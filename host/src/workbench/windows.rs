//! The windows the app draws in: its own, unless it asked for none, and one for each `g-window`
//! in the tree. They all draw from the one app and the one tree, so state is shared between
//! them as it is between any two components.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Entity, FocusHandle, IntoElement, Render, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, size,
};

use super::Workbench;
use crate::render::Painter;
use crate::tree::{Kind, NodeId, ROOT, Tree};

/// A `g-window` opens at this size when it does not say.
const DEFAULT_WIDTH: f32 = 400.0;
const DEFAULT_HEIGHT: f32 = 300.0;

/// A window's own view: it draws the part of the tree under `root` through the workbench.
/// The windows are what keeps the workbench alive.
pub struct WindowView {
    workbench: Entity<Workbench>,
    root: NodeId,
}

impl Render for WindowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root;
        self.workbench.update(cx, |workbench, cx| workbench.render_window(root, window, cx))
    }
}

/// A window that is open, and the node it draws.
pub(crate) struct OpenWindow {
    pub root: NodeId,
    pub handle: WindowHandle<WindowView>,
}

/// The windows' bookkeeping, apart from the rest of the workbench.
#[derive(Default)]
pub(crate) struct Windows {
    /// In the order they opened; the first is the one timers and scripted steps run in.
    pub open: Vec<OpenWindow>,
    /// `g-window`s whose window is being opened.
    pub opening: HashSet<NodeId>,
    /// `g-window`s whose window the user or the compositor closed. They stay closed until
    /// the app removes the node.
    pub closed: HashSet<NodeId>,
    /// The app's id, which a `g-window` takes when it does not name its own.
    pub app_id: Option<String>,
}

impl Workbench {
    /// Opens a window that draws the children of `root`. It must not run inside an update of
    /// the workbench: the window draws once as it opens.
    pub fn open_window(
        this: &Entity<Workbench>,
        root: NodeId,
        options: WindowOptions,
        cx: &mut App,
    ) -> Option<WindowHandle<WindowView>> {
        let workbench = this.clone();
        let opened = cx.open_window(options, |window, cx| {
            this.update(cx, |this, cx| this.attach_window(root, window, cx));
            cx.new(|cx| {
                // The workbench is not a view, so its changes reach the window through this one.
                cx.observe(this, |_, _, cx| cx.notify()).detach();
                WindowView { workbench, root }
            })
        });
        let handle = match opened {
            Ok(handle) => handle,
            Err(error) => {
                eprintln!("svelte-gpui: cannot open a window: {error:#}");
                return None;
            }
        };
        this.update(cx, |this, _| {
            this.windows.opening.remove(&root);
            this.windows.open.push(OpenWindow { root, handle });
        });
        Some(handle)
    }

    /// Prepares the workbench for a window that is opening: what holds focus in it, and how
    /// it closes.
    fn attach_window(&mut self, root: NodeId, window: &mut Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.interaction.window_focus.borrow_mut().insert(root, focus);
        self.interaction.window_roots.borrow_mut().insert(window.window_handle().window_id(), root);
        if root == ROOT {
            self.watch_window(window, cx);
            return;
        }
        // A `g-window` the user closes tells the app, which decides whether its node goes.
        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |_window, cx| {
            this.update(cx, |workbench, cx| workbench.window_closed(root, cx)).ok();
            true
        });
    }

    fn window_closed(&mut self, root: NodeId, cx: &mut Context<Self>) {
        self.windows.closed.insert(root);
        self.forget_window(root);
        self.emit(root, "close", "{}".to_string(), cx);
    }

    fn forget_window(&mut self, root: NodeId) {
        self.windows.open.retain(|open| open.root != root);
        self.interaction.window_focus.borrow_mut().remove(&root);
        self.interaction.window_roots.borrow_mut().retain(|_, drawn| *drawn != root);
    }

    /// Runs `action` on the workbench in its primary window. `None` while it has no window.
    pub fn in_primary_window<R>(
        this: &Entity<Workbench>,
        cx: &mut App,
        action: impl FnOnce(&mut Workbench, &mut Window, &mut Context<Workbench>) -> R,
    ) -> Option<R> {
        let handle = this.read(cx).primary_window()?;
        handle.update(cx, |_, window, cx| this.update(cx, |workbench, cx| action(workbench, window, cx))).ok()
    }

    /// The window that timers, scripted steps and reports run in.
    pub fn primary_window(&self) -> Option<WindowHandle<WindowView>> {
        self.windows.open.first().map(|open| open.handle)
    }

    /// The window an element is drawn in.
    pub(super) fn window_of(&self, id: NodeId) -> Option<WindowHandle<WindowView>> {
        let tree = self.svelte.tree.borrow();
        let root = window_root(&tree, id)?;
        self.windows.open.iter().find(|open| open.root == root).map(|open| open.handle)
    }

    /// Runs `action` in `target`: in `current` when that is the one, since a window that is
    /// already being updated cannot be reached through its handle.
    pub(super) fn in_window<R>(
        target: WindowHandle<WindowView>,
        current: &mut Window,
        cx: &mut App,
        action: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> Option<R> {
        if current.window_handle().window_id() == target.window_id() {
            return Some(action(current, cx));
        }
        target.update(cx, |_, window, cx| action(window, cx)).ok()
    }

    /// The element with keyboard focus: in the active window, or, while none of the app's
    /// windows is active, the one that had it, as long as it still has it in its window.
    pub(super) fn focused_anywhere(&self, current: &mut Window, cx: &mut App) -> Option<NodeId> {
        let handles: Vec<(NodeId, FocusHandle)> =
            self.interaction.focus.borrow().iter().map(|(id, handle)| (*id, handle.clone())).collect();
        let mut kept = None;
        for open in &self.windows.open {
            let found = Self::in_window(open.handle, current, cx, |window, _| {
                let focused = handles.iter().find(|(_, handle)| handle.is_focused(window)).map(|(id, _)| *id);
                (window.is_window_active(), focused)
            });
            match found {
                Some((true, focused)) => return focused,
                Some((false, Some(focused))) if Some(focused) == self.focused => kept = Some(focused),
                _ => {}
            }
        }
        kept
    }

    /// Opens a window for each `g-window` new to the tree and closes the windows whose node
    /// left it. The windows open and close after this update, not inside it.
    pub(super) fn sync_windows(&mut self, cx: &mut Context<Self>) {
        let wanted = {
            let tree = self.svelte.tree.borrow();
            mounted_windows(&tree)
        };
        self.windows.closed.retain(|root| wanted.contains(root));
        let gone: Vec<OpenWindow> = {
            let (gone, kept) = std::mem::take(&mut self.windows.open)
                .into_iter()
                .partition(|open| open.root != ROOT && !wanted.contains(&open.root));
            self.windows.open = kept;
            gone
        };
        for window in gone {
            self.forget_window(window.root);
            cx.defer(move |cx| {
                window.handle.update(cx, |_, window, _| window.remove_window()).ok();
            });
        }
        let this = cx.entity();
        for root in wanted {
            let known = self.windows.open.iter().any(|open| open.root == root)
                || self.windows.opening.contains(&root)
                || self.windows.closed.contains(&root);
            if known {
                continue;
            }
            self.windows.opening.insert(root);
            let this = this.clone();
            // Read when it opens, so that what was set after this update, like the app id, counts.
            cx.defer(move |cx| {
                let (attributes, app_id) = {
                    let workbench = this.read(cx);
                    let tree = workbench.svelte.tree.borrow();
                    let attributes = tree.nodes.get(&root).map(|node| node.attributes.clone()).unwrap_or_default();
                    (attributes, workbench.windows.app_id.clone())
                };
                let options = window_options(&attributes, app_id, cx);
                Workbench::open_window(&this, root, options, cx);
            });
        }
    }

    /// Draws one window: the app's own, or a `g-window`'s.
    pub fn render_window(&mut self, root: NodeId, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let started = Instant::now();
        let viewport = window.viewport_size();
        self.interaction.viewport.set((f32::from(viewport.width), f32::from(viewport.height)));
        self.interaction.scale.set(window.scale_factor());
        let gone: Vec<_> = std::mem::take(&mut self.svelte.tree.borrow_mut().dropped_canvases);
        for surface in gone {
            for image in surface.borrow_mut().release() {
                window.drop_image(image).ok();
            }
        }
        let tree = self.svelte.tree.clone();
        let tree = tree.borrow();
        let painter = Painter::new(&tree, &self.interaction, None);
        let document = painter.document(root, cx);
        let primary = self.primary_window().map(|handle| handle.window_id());
        let overlay = match primary == Some(window.window_handle().window_id()) {
            true => self.error_overlay(window, cx),
            false => None,
        };
        let document = match overlay {
            Some(overlay) => div().size_full().relative().child(document).child(overlay).into_any_element(),
            None => document,
        };
        let wanted = self.frame_target;
        if wanted > 0 && self.frame_times.len() < wanted {
            self.frame_times.push(Instant::now());
            window.request_animation_frame();
        }
        if self.interaction.animating.replace(false) {
            window.request_animation_frame();
        }
        // What this frame laid out is reported, and animation frames run, when the
        // display next draws.
        self.tick_next_frame(window, cx);
        let elapsed = started.elapsed().as_secs_f64();
        {
            let mut frames = self.svelte.frames.borrow_mut();
            frames.push_back((Instant::now(), elapsed * 1000.0));
            if frames.len() > 1000 {
                frames.pop_front();
            }
        }
        self.stats.renders += 1;
        self.stats.render_seconds += elapsed;
        self.stats.worst_render_seconds = self.stats.worst_render_seconds.max(elapsed);
        document
    }
}

/// The `g-window` an element sits in, or the root for the app's own window.
pub(super) fn window_root(tree: &Tree, id: NodeId) -> Option<NodeId> {
    let mut current = Some(id);
    while let Some(node) = current {
        if node == ROOT || is_window(tree, node) {
            return Some(node);
        }
        current = tree.nodes.get(&node)?.parent;
    }
    None
}

fn is_window(tree: &Tree, id: NodeId) -> bool {
    matches!(tree.nodes.get(&id).map(|node| &node.kind), Some(Kind::Element { tag }) if tag == "g-window")
}

/// The `g-window`s that are in the tree, rather than built and not yet inserted.
fn mounted_windows(tree: &Tree) -> HashSet<NodeId> {
    let mut found = HashSet::new();
    let mut pending = vec![ROOT];
    while let Some(id) = pending.pop() {
        let Some(node) = tree.nodes.get(&id) else {
            continue;
        };
        if id != ROOT && is_window(tree, id) {
            found.insert(id);
        }
        pending.extend(node.children.iter().copied());
    }
    found
}

/// How a `g-window` opens, from its attributes: `title`, `width`, `height` and `app-id`, which
/// is the app's own unless it says.
fn window_options(attributes: &HashMap<String, String>, app_id: Option<String>, cx: &mut App) -> WindowOptions {
    let number = |name: &str| attributes.get(name).and_then(|value| value.parse::<f32>().ok());
    let width = number("width").unwrap_or(DEFAULT_WIDTH);
    let height = number("height").unwrap_or(DEFAULT_HEIGHT);
    let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: attributes
            .get("title")
            .map(|title| TitlebarOptions { title: Some(title.clone().into()), ..Default::default() }),
        app_id: attributes.get("app-id").cloned().or(app_id),
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    }
}
