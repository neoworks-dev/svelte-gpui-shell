//! The windows the app draws in: its own, unless it asked for none, and one for each `g-window`
//! in the tree. They all draw from the one app and the one tree, so state is shared between
//! them as it is between any two components.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use gpui::layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions};
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Entity, FocusHandle, IntoElement, Pixels, Render, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, point, prelude::*,
    px, size,
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
    /// The attributes that can change an open window, as it was last given them.
    pub applied: HashMap<String, String>,
    /// The input region the window was last given: `None` while the whole surface takes input.
    pub input_region: Option<Vec<super::input_region::Rectangle>>,
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
    /// The monitors as the app was last told, so it hears only changes.
    pub reported_monitors: Option<String>,
}

/// The attributes an open window follows: a layer surface's exclusive zone and keyboard
/// mode, and its size.
const LIVE_ATTRIBUTES: [&str; 4] = ["exclusive-zone", "keyboard", "width", "height"];

fn live_attributes(attributes: &HashMap<String, String>) -> HashMap<String, String> {
    let live = attributes.iter().filter(|(name, _)| LIVE_ATTRIBUTES.contains(&name.as_str()));
    live.map(|(name, value)| (name.clone(), value.clone())).collect()
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
            let applied = this.svelte.tree.borrow().nodes.get(&root).map(|node| live_attributes(&node.attributes));
            this.windows.open.push(OpenWindow {
                root,
                handle,
                applied: applied.unwrap_or_default(),
                input_region: None,
            });
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
        self.follow_attributes(cx);
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
                match window_options(&attributes, app_id, cx) {
                    Some(options) => {
                        Workbench::open_window(&this, root, options, cx);
                    }
                    None => this.update(cx, |this, _| {
                        this.windows.opening.remove(&root);
                        this.windows.closed.insert(root);
                    }),
                }
            });
        }
    }

    /// Carries changes to an open `g-window`'s exclusive zone, keyboard mode or size over to
    /// its window, after this update.
    fn follow_attributes(&mut self, cx: &mut Context<Self>) {
        let tree = self.svelte.tree.borrow();
        for open in &mut self.windows.open {
            let Some(node) = tree.nodes.get(&open.root) else {
                continue;
            };
            let now = live_attributes(&node.attributes);
            if now == open.applied || open.root == ROOT {
                continue;
            }
            let before = std::mem::replace(&mut open.applied, now.clone());
            let handle = open.handle;
            cx.defer(move |cx| {
                handle.update(cx, |_, window, _| apply_changes(&before, &now, window)).ok();
            });
        }
    }

    /// Tells the app which monitors are connected, when that changed since the last time.
    pub(super) fn report_monitors(&mut self, cx: &mut Context<Self>) {
        let list: Vec<serde_json::Value> = cx
            .displays()
            .iter()
            .filter_map(|display| {
                let bounds = display.bounds();
                Some(serde_json::json!({
                    "name": display.name()?,
                    "x": f32::from(bounds.origin.x),
                    "y": f32::from(bounds.origin.y),
                    "width": f32::from(bounds.size.width),
                    "height": f32::from(bounds.size.height),
                }))
            })
            .collect();
        let json = serde_json::Value::Array(list).to_string();
        if self.windows.reported_monitors.as_ref() == Some(&json) {
            return;
        }
        self.svelte.monitors(&json);
        self.windows.reported_monitors = Some(json);
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

pub(super) fn is_window(tree: &Tree, id: NodeId) -> bool {
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

/// Applies what changed between two sets of live attributes to an open window.
fn apply_changes(before: &HashMap<String, String>, now: &HashMap<String, String>, window: &mut Window) {
    let changed = |name: &str| before.get(name) != now.get(name);
    if changed("exclusive-zone") {
        let zone = now.get("exclusive-zone").and_then(|zone| zone.parse::<f32>().ok()).unwrap_or(0.0);
        window.set_exclusive_zone(px(zone));
    }
    if changed("keyboard") {
        window.set_keyboard_interactivity(keyboard(now.get("keyboard").map(String::as_str)));
    }
    if changed("width") || changed("height") {
        let number = |name: &str| now.get(name).and_then(|value| value.parse::<f32>().ok()).unwrap_or(0.0);
        window.resize(size(px(number("width")), px(number("height"))));
    }
}

/// How a `g-window` opens, from its attributes: `title`, `width`, `height`, `app-id` (the app's
/// own unless it says), `monitor` (a display's name) and, for a layer surface, `layer` with
/// `anchor`, `exclusive-zone`, `margin`, `keyboard` and `namespace`. `None` when it names a
/// monitor that is not connected: it does not open elsewhere instead.
fn window_options(attributes: &HashMap<String, String>, app_id: Option<String>, cx: &mut App) -> Option<WindowOptions> {
    let number = |name: &str| attributes.get(name).and_then(|value| value.parse::<f32>().ok());
    let display = match attributes.get("monitor") {
        Some(name) => {
            let found = cx.displays().into_iter().find(|display| display.name().as_deref() == Some(name.as_str()));
            if found.is_none() {
                eprintln!("svelte-gpui: no monitor named {name:?}; the window does not open");
                return None;
            }
            found
        }
        None => None,
    };
    let layer = attributes.get("layer").and_then(|layer| layer_options(attributes, layer));
    // A layer surface sized 0 along an axis is stretched there by the compositor.
    let (default_width, default_height) = match layer {
        Some(_) => (0.0, 0.0),
        None => (DEFAULT_WIDTH, DEFAULT_HEIGHT),
    };
    let width = number("width").unwrap_or(default_width);
    let height = number("height").unwrap_or(default_height);
    let bounds = match &layer {
        Some(_) => Bounds { origin: point(px(0.0), px(0.0)), size: size(px(width), px(height)) },
        None => Bounds::centered(display.as_ref().map(|display| display.id()), size(px(width), px(height)), cx),
    };
    Some(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: attributes
            .get("title")
            .map(|title| TitlebarOptions { title: Some(title.clone().into()), ..Default::default() }),
        app_id: attributes.get("app-id").cloned().or(app_id),
        display_id: display.map(|display| display.id()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: match layer {
            Some(layer) => WindowKind::LayerShell(layer),
            None => WindowKind::Normal,
        },
        ..Default::default()
    })
}

/// A layer surface's options. `None` for a `layer` it does not know.
fn layer_options(attributes: &HashMap<String, String>, layer: &str) -> Option<LayerShellOptions> {
    let layer = match layer {
        "background" => Layer::Background,
        "bottom" => Layer::Bottom,
        "top" => Layer::Top,
        "overlay" => Layer::Overlay,
        _ => {
            eprintln!("svelte-gpui: a Window's layer is background, bottom, top or overlay, not {layer:?}");
            return None;
        }
    };
    let mut anchor = Anchor::empty();
    for edge in attributes.get("anchor").map(String::as_str).unwrap_or("").split_whitespace() {
        anchor |= match edge {
            "top" => Anchor::TOP,
            "bottom" => Anchor::BOTTOM,
            "left" => Anchor::LEFT,
            "right" => Anchor::RIGHT,
            _ => Anchor::empty(),
        };
    }
    Some(LayerShellOptions {
        namespace: attributes.get("namespace").cloned().unwrap_or_else(|| "svelte-gpui".to_string()),
        layer,
        anchor,
        exclusive_zone: attributes.get("exclusive-zone").and_then(|zone| zone.parse::<f32>().ok()).map(px),
        margin: attributes.get("margin").and_then(|margin| parse_margin(margin)),
        keyboard_interactivity: keyboard(attributes.get("keyboard").map(String::as_str)),
        ..Default::default()
    })
}

/// How a layer surface takes the keyboard: `none` (the default), `exclusive` or `on-demand`.
pub(super) fn keyboard(value: Option<&str>) -> KeyboardInteractivity {
    match value {
        Some("exclusive") => KeyboardInteractivity::Exclusive,
        Some("on-demand") => KeyboardInteractivity::OnDemand,
        _ => KeyboardInteractivity::None,
    }
}

/// `top right bottom left`, or one number for all four.
fn parse_margin(value: &str) -> Option<(Pixels, Pixels, Pixels, Pixels)> {
    let numbers: Vec<f32> = value.split_whitespace().filter_map(|part| part.parse().ok()).collect();
    match numbers.as_slice() {
        [all] => Some((px(*all), px(*all), px(*all), px(*all))),
        [top, right, bottom, left] => Some((px(*top), px(*right), px(*bottom), px(*left))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    }

    #[test]
    fn layer_options_read_anchor_zone_and_keyboard() {
        let given = attributes(&[("anchor", "top left right"), ("exclusive-zone", "32"), ("keyboard", "on-demand")]);
        let options = layer_options(&given, "top").unwrap();
        assert_eq!(options.layer, Layer::Top);
        assert_eq!(options.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(options.exclusive_zone, Some(px(32.0)));
        assert_eq!(options.keyboard_interactivity, KeyboardInteractivity::OnDemand);
        assert_eq!(options.namespace, "svelte-gpui");
    }

    #[test]
    fn an_unknown_layer_is_not_a_layer_surface() {
        assert!(layer_options(&attributes(&[]), "sideways").is_none());
    }

    #[test]
    fn margins_are_one_number_or_four() {
        assert_eq!(parse_margin("8"), Some((px(8.0), px(8.0), px(8.0), px(8.0))));
        assert_eq!(parse_margin("1 2 3 4"), Some((px(1.0), px(2.0), px(3.0), px(4.0))));
        assert_eq!(parse_margin("1 2"), None);
    }

    #[test]
    fn only_live_attributes_are_followed() {
        let given = attributes(&[("height", "32"), ("layer", "top"), ("keyboard", "none")]);
        let live = live_attributes(&given);
        assert_eq!(live.len(), 2);
        assert!(!live.contains_key("layer"));
    }
}
