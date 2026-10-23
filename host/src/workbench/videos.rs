//! The players behind `g-video` elements: started when an element gets a source, kept in step with
//! its attributes, and stopped when it goes.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::Context;
use serde_json::Value;

use super::Workbench;
use crate::tree::{NodeId, ROOT, Tree};
use crate::video::{Player, Settings};

impl Workbench {
    /// Brings the players in step with the videos that were created, changed or removed.
    pub(super) fn sync_videos(&mut self, cx: &mut Context<Self>) {
        let (changed, dropped) = {
            let mut tree = self.svelte.tree.borrow_mut();
            (std::mem::take(&mut tree.changed_videos), std::mem::take(&mut tree.dropped_videos))
        };
        for id in dropped {
            self.retire_video(id);
        }
        for id in changed {
            self.follow_video(id, cx);
        }
    }

    fn follow_video(&mut self, id: NodeId, cx: &mut Context<Self>) {
        let wanted = {
            let tree = self.svelte.tree.borrow();
            let mounted = is_mounted(&tree, id);
            let node = tree.nodes.get(&id).filter(|_| mounted);
            node.and_then(|node| Settings::from_attributes(&node.attributes, self.interaction.images.base()))
        };
        let existing = self.interaction.videos.borrow().get(&id).cloned();
        if let (Some(player), Some(wanted)) = (&existing, &wanted) {
            if !player.borrow().settings().needs_restart(wanted) {
                player.borrow_mut().follow(wanted);
                self.touch(id, cx);
                return;
            }
        }
        if existing.is_some() {
            self.retire_video(id);
        }
        let Some(wanted) = wanted else {
            return;
        };
        match Player::start(id, wanted, self.svelte.io.sink()) {
            Ok(player) => {
                self.interaction.videos.borrow_mut().insert(id, Rc::new(RefCell::new(player)));
                self.touch(id, cx);
            }
            Err(message) => {
                eprintln!("svelte-gpui: video: {message}");
                let detail = serde_json::json!({ "message": message }).to_string();
                self.emit(id, "error", detail, cx);
            }
        }
    }

    /// Stops a player. Its images are freed by the next window that draws.
    fn retire_video(&mut self, id: NodeId) {
        let removed = self.interaction.videos.borrow_mut().remove(&id);
        if let Some(player) = removed {
            self.interaction.retired_videos.borrow_mut().push(player);
        }
    }

    /// What a player reported: a new frame to draw, or something the app listens for.
    pub(super) fn deliver_video_event(&mut self, node: NodeId, payload: &Value, cx: &mut Context<Self>) {
        match payload["name"].as_str() {
            Some("frame") => self.touch(node, cx),
            Some(name) => {
                let detail = payload.get("detail").cloned().unwrap_or_else(|| serde_json::json!({})).to_string();
                self.emit(node, name, detail, cx);
            }
            None => {}
        }
    }
}

/// Whether a node is in the document, rather than built and not yet inserted.
fn is_mounted(tree: &Tree, id: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(node) = current {
        if node == ROOT {
            return true;
        }
        current = tree.nodes.get(&node).and_then(|found| found.parent);
    }
    false
}
