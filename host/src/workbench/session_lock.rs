//! The session lock, as the app sees it. A `Window` with `lock` is a lock surface on one
//! monitor, and the session stays locked while any of them exists. The compositor confirms the
//! lock, or ends it, some time after the first surface opens; this module turns what the
//! windows say into one state the app reads from `sessionLock`.

use std::collections::HashSet;

use gpui::{WindowHandle, WindowOptions};

use super::Workbench;
use super::windows::{WindowView, mounted_windows};
use crate::tree::{NodeId, Tree};

/// Lock surfaces open only in a process that has this set to `1`. A locker that is wrong traps
/// the session, so a run that did not ask for it, such as a test or a smoke, never takes it.
const ALLOW_LOCK_VARIABLE: &str = "SVELTE_GPUI_ALLOW_SESSION_LOCK";

/// Whether this process may open lock surfaces.
pub(super) fn lock_allowed() -> bool {
    std::env::var(ALLOW_LOCK_VARIABLE).is_ok_and(|value| value == "1")
}

/// Where the lock is, from the app's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LockState {
    /// No lock surface is wanted.
    Unlocked,
    /// Surfaces are wanted and the compositor has not confirmed the lock yet.
    Locking,
    /// The compositor confirmed the lock: the session is locked, and the surfaces are what shows.
    Locked,
    /// Surfaces are wanted but none is left: the compositor refused or ended the lock, or this
    /// process refused to open one. It goes back to `Unlocked` when the app removes them.
    Finished,
}

impl LockState {
    pub(crate) fn name(self) -> &'static str {
        match self {
            LockState::Unlocked => "unlocked",
            LockState::Locking => "locking",
            LockState::Locked => "locked",
            LockState::Finished => "finished",
        }
    }
}

/// What the windows say, counted over the `g-window`s that ask for a lock surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Observed {
    /// Lock windows in the tree.
    pub wanted: usize,
    /// Lock windows being opened.
    pub opening: usize,
    /// Lock windows that are open.
    pub open: usize,
    /// Of those, the ones the compositor has confirmed the lock for.
    pub confirmed: usize,
}

pub(crate) fn lock_state(seen: Observed) -> LockState {
    if seen.wanted == 0 {
        return LockState::Unlocked;
    }
    if seen.confirmed > 0 {
        return LockState::Locked;
    }
    if seen.opening + seen.open > 0 {
        return LockState::Locking;
    }
    LockState::Finished
}

/// The `g-window`s in the tree that ask for a lock surface.
fn lock_windows(tree: &Tree) -> HashSet<NodeId> {
    let windows = mounted_windows(tree);
    windows
        .into_iter()
        .filter(|id| tree.nodes.get(id).is_some_and(|node| node.attributes.contains_key("lock")))
        .collect()
}

impl Workbench {
    /// Works out the lock's state and tells the app when it changed. The windows are asked after
    /// this tick, not inside it.
    pub(super) fn report_session_lock(&mut self, cx: &mut gpui::Context<Self>) {
        let wanted = lock_windows(&self.svelte.tree.borrow());
        if wanted.is_empty() {
            self.set_lock_state(LockState::Unlocked);
            return;
        }
        let opening = self.windows.opening.iter().filter(|root| wanted.contains(root)).count();
        let open: Vec<(NodeId, WindowHandle<WindowView>)> = self
            .windows
            .open
            .iter()
            .filter(|open| wanted.contains(&open.root))
            .map(|open| (open.root, open.handle))
            .collect();
        let this = cx.weak_entity();
        let wanted = wanted.len();
        cx.defer(move |cx| {
            let mut gone = Vec::new();
            let mut confirmed = 0;
            for (root, handle) in &open {
                match handle.update(cx, |_, window, _| window.session_lock_confirmed()) {
                    Ok(Some(true)) => confirmed += 1,
                    Ok(_) => {}
                    Err(_) => gone.push(*root),
                }
            }
            let seen = Observed { wanted, opening, open: open.len() - gone.len(), confirmed };
            this.update(cx, |workbench, cx| {
                for root in gone {
                    workbench.window_closed(root, cx);
                }
                workbench.set_lock_state(lock_state(seen));
            })
            .ok();
        });
    }

    fn set_lock_state(&mut self, state: LockState) {
        let reported = self.windows.reported_lock.unwrap_or(LockState::Unlocked);
        if reported == state && self.windows.reported_lock.is_some() {
            return;
        }
        self.windows.reported_lock = Some(state);
        self.svelte.session_lock(state.name());
    }
}

/// How a lock window opens: on the monitor it names, as a lock surface the compositor sizes.
/// `None` when this process refuses to lock, or when no monitor is named.
pub(super) fn lock_options(display: Option<gpui::DisplayId>) -> Option<WindowOptions> {
    if !lock_allowed() {
        eprintln!("svelte-gpui: a lock Window does not open unless {ALLOW_LOCK_VARIABLE}=1 is set");
        return None;
    }
    let Some(display) = display else {
        eprintln!("svelte-gpui: a lock Window needs a monitor");
        return None;
    };
    let options = gpui::layer_shell::LayerShellOptions {
        namespace: gpui::layer_shell::SESSION_LOCK_NAMESPACE.to_string(),
        ..Default::default()
    };
    Some(WindowOptions {
        window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::default())),
        display_id: Some(display),
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        kind: gpui::WindowKind::LayerShell(options),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(wanted: usize, opening: usize, open: usize, confirmed: usize) -> LockState {
        lock_state(Observed { wanted, opening, open, confirmed })
    }

    #[test]
    fn nothing_wanted_is_unlocked_whatever_is_left_open() {
        assert_eq!(seen(0, 0, 0, 0), LockState::Unlocked);
        assert_eq!(seen(0, 0, 2, 2), LockState::Unlocked);
    }

    #[test]
    fn surfaces_that_are_opening_or_open_without_the_compositors_word_are_locking() {
        assert_eq!(seen(2, 2, 0, 0), LockState::Locking);
        assert_eq!(seen(2, 1, 1, 0), LockState::Locking);
        assert_eq!(seen(2, 0, 2, 0), LockState::Locking);
    }

    #[test]
    fn one_confirmed_surface_is_locked_even_while_others_open() {
        assert_eq!(seen(3, 1, 2, 1), LockState::Locked);
        assert_eq!(seen(3, 0, 3, 3), LockState::Locked);
    }

    #[test]
    fn wanted_surfaces_that_all_went_are_finished_not_unlocked() {
        assert_eq!(seen(2, 0, 0, 0), LockState::Finished);
    }

    #[test]
    fn the_names_are_what_the_app_reads() {
        let names: Vec<&str> = [LockState::Unlocked, LockState::Locking, LockState::Locked, LockState::Finished]
            .iter()
            .map(|state| state.name())
            .collect();
        assert_eq!(names, ["unlocked", "locking", "locked", "finished"]);
    }
}
