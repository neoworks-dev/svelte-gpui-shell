//! What stands in for the player in a host built without the `video` feature.

use std::sync::Arc;

use gpui::RenderImage;

use super::Settings;
use crate::io::Sink;
use crate::tree::NodeId;

pub struct Player {
    settings: Settings,
    pub stale: Vec<Arc<RenderImage>>,
}

impl Player {
    pub fn start(_node: NodeId, _settings: Settings, _sink: Sink) -> Result<Player, String> {
        Err("this host was built without the `video` feature".to_string())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn follow(&mut self, _wanted: &Settings) {}

    pub fn picture(&mut self, _pixels: (u32, u32)) -> Option<(Arc<RenderImage>, (u32, u32))> {
        None
    }

    pub fn release(&mut self) -> Vec<Arc<RenderImage>> {
        Vec::new()
    }
}
