//! The part of a layer surface that takes the pointer. Elements marked `input` are that part;
//! everything else in the window is click-through. A window without any marked element takes
//! input everywhere, and one whose marked elements are all empty takes none.

use std::collections::HashMap;

use gpui::{Bounds, Pixels, Window, point, px, size};

use super::Workbench;
use super::windows::is_window;
use crate::tree::{NodeId, Tree};

/// A rectangle in whole logical pixels: left, top, width, height.
pub(crate) type Rectangle = (i32, i32, i32, i32);

/// The region the elements marked `input` under `root` ask for, or `None` when there are none.
/// The rectangles are rounded outwards, so a fractional edge never leaves a pixel click-through
/// that the element covers, and elements with no area add nothing.
pub(crate) fn requested_region(
    tree: &Tree,
    root: NodeId,
    bounds: &HashMap<NodeId, Bounds<Pixels>>,
) -> Option<Vec<Rectangle>> {
    let marked = marked_elements(tree, root);
    if marked.is_empty() {
        return None;
    }
    let mut rectangles: Vec<Rectangle> =
        marked.iter().filter_map(|id| bounds.get(id)).filter_map(|bounds| rounded_outwards(*bounds)).collect();
    rectangles.sort();
    rectangles.dedup();
    Some(rectangles)
}

/// The elements marked `input` that are drawn in the window of `root`, in tree order.
fn marked_elements(tree: &Tree, root: NodeId) -> Vec<NodeId> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(node) = tree.nodes.get(&id) else {
            continue;
        };
        if id != root && is_window(tree, id) {
            continue;
        }
        if node.attributes.contains_key("input") {
            found.push(id);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    found
}

fn rounded_outwards(bounds: Bounds<Pixels>) -> Option<Rectangle> {
    let left = f32::from(bounds.left()).floor() as i32;
    let top = f32::from(bounds.top()).floor() as i32;
    let right = f32::from(bounds.right()).ceil() as i32;
    let bottom = f32::from(bounds.bottom()).ceil() as i32;
    if f32::from(bounds.size.width) <= 0.0 || f32::from(bounds.size.height) <= 0.0 {
        return None;
    }
    Some((left, top, right - left, bottom - top))
}

fn to_bounds(rectangle: &Rectangle) -> Bounds<Pixels> {
    let (left, top, width, height) = *rectangle;
    Bounds { origin: point(px(left as f32), px(top as f32)), size: size(px(width as f32), px(height as f32)) }
}

impl Workbench {
    /// Hands every open window the input region its marked elements ask for, when that differs
    /// from the one it has. The windows are updated after this tick, not inside it.
    pub(super) fn follow_input_regions(&mut self, cx: &mut gpui::Context<Self>) {
        let tree = self.svelte.tree.borrow();
        let bounds = self.interaction.bounds.borrow();
        for open in &mut self.windows.open {
            let wanted = requested_region(&tree, open.root, &bounds);
            if wanted == open.input_region {
                continue;
            }
            open.input_region = wanted.clone();
            let handle = open.handle;
            cx.defer(move |cx| {
                handle.update(cx, |_, window, _| apply_region(&wanted, window)).ok();
            });
        }
    }
}

fn apply_region(region: &Option<Vec<Rectangle>>, window: &mut Window) {
    match region {
        Some(rectangles) => {
            let bounds: Vec<Bounds<Pixels>> = rectangles.iter().map(to_bounds).collect();
            window.set_input_region(Some(&bounds));
        }
        None => window.set_input_region(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{CREATE_ELEMENT, INSERT, ROOT, SET_ATTRIBUTE};

    /// A `g-window` (2) holding two plain views (3, 4) and a view (5); another `g-window` (6)
    /// holds a view (7). The views in `marked` carry `input`.
    fn tree(marked: &[NodeId]) -> Tree {
        let mut tree = Tree::new();
        let mut batch: Vec<String> = [2, 6]
            .into_iter()
            .map(|id| format!("[{CREATE_ELEMENT},{id},\"g-window\"]"))
            .chain([3, 4, 5, 7].into_iter().map(|id| format!("[{CREATE_ELEMENT},{id},\"g-view\"]")))
            .collect();
        for (parent, child) in [(ROOT, 2), (ROOT, 6), (2, 3), (2, 4), (2, 5), (6, 7)] {
            batch.push(format!("[{INSERT},{parent},{child},0]"));
        }
        for id in marked {
            batch.push(format!("[{SET_ATTRIBUTE},{id},\"input\",\"\"]"));
        }
        tree.apply(&format!("[{}]", batch.join(",")));
        tree
    }

    fn at(left: f32, top: f32, width: f32, height: f32) -> Bounds<Pixels> {
        Bounds { origin: point(px(left), px(top)), size: size(px(width), px(height)) }
    }

    #[test]
    fn a_window_without_marked_elements_takes_input_everywhere() {
        let bounds = HashMap::from([(3, at(0.0, 0.0, 500.0, 500.0))]);
        assert_eq!(requested_region(&tree(&[]), 2, &bounds), None);
        assert_eq!(requested_region(&tree(&[7]), 2, &bounds), None);
    }

    #[test]
    fn the_region_is_the_marked_elements_of_that_window_only() {
        let bounds = HashMap::from([
            (3, at(0.0, 0.0, 500.0, 500.0)),
            (4, at(10.0, 20.0, 100.0, 50.0)),
            (5, at(200.0, 0.0, 30.0, 30.0)),
            (7, at(0.0, 0.0, 9.0, 9.0)),
        ]);
        let marked = tree(&[4, 5, 7]);
        assert_eq!(requested_region(&marked, 2, &bounds), Some(vec![(10, 20, 100, 50), (200, 0, 30, 30)]));
        assert_eq!(requested_region(&marked, 6, &bounds), Some(vec![(0, 0, 9, 9)]));
    }

    #[test]
    fn marked_elements_with_no_area_leave_the_window_click_through() {
        let bounds = HashMap::from([(4, at(10.0, 20.0, 0.0, 0.0)), (5, at(30.0, 30.0, 0.0, 40.0))]);
        assert_eq!(requested_region(&tree(&[4, 5]), 2, &bounds), Some(vec![]));
        assert_eq!(requested_region(&tree(&[4]), 2, &HashMap::new()), Some(vec![]));
    }

    #[test]
    fn fractional_edges_round_outwards() {
        let bounds = HashMap::from([(4, at(10.4, 20.6, 100.2, 50.0))]);
        assert_eq!(requested_region(&tree(&[4]), 2, &bounds), Some(vec![(10, 20, 101, 51)]));
    }
}
