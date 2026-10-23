//! `g-video`: a video file or stream played through GStreamer, drawn as a picture that is
//! replaced as new frames come in.
//!
//! The pipeline decodes with whatever the system offers (VA-API decoders first), converts to
//! BGRA and scales to the size the picture is drawn at, so a 4K video on a 1080p output costs
//! the copy of a 1080p frame and not of a 4K one. Frames arrive on GStreamer's own threads and
//! are kept in [`Shared`]; the window is woken once per frame it has not yet taken, and draws
//! the newest.

use std::collections::HashMap;

#[cfg(feature = "video")]
mod player;
#[cfg(not(feature = "video"))]
mod unavailable;

#[cfg(feature = "video")]
pub use player::Player;
#[cfg(not(feature = "video"))]
pub use unavailable::Player;

/// How the picture fills its box, as for an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// All of the video shows, with room left over on one axis.
    Contain,
    /// The box is full, and what sticks out is cut off.
    Cover,
    /// The video is stretched to the box.
    Fill,
}

impl Fit {
    pub fn parse(value: &str) -> Fit {
        match value {
            "cover" => Fit::Cover,
            "fill" => Fit::Fill,
            _ => Fit::Contain,
        }
    }
}

/// What a `g-video`'s attributes ask for.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub source: String,
    pub looping: bool,
    pub muted: bool,
    pub paused: bool,
    pub rate: f64,
    pub fit: Fit,
    pub max_fps: Option<u32>,
}

impl Settings {
    /// Reads the attributes of a `g-video`. `base` is where a relative path is looked up.
    /// `None` without a `src`.
    pub fn from_attributes(attributes: &HashMap<String, String>, base: &std::path::Path) -> Option<Settings> {
        let source = attributes.get("src").filter(|source| !source.is_empty())?;
        let flag = |name: &str, default: bool| match attributes.get(name).map(String::as_str) {
            Some("false") => false,
            Some(_) => true,
            None => default,
        };
        let rate = attributes.get("rate").and_then(|rate| rate.parse::<f64>().ok()).unwrap_or(1.0);
        Some(Settings {
            source: resolve(source, base),
            looping: flag("loop", true),
            muted: flag("muted", true),
            paused: attributes.contains_key("paused") && flag("paused", false),
            rate: clamp_rate(rate),
            fit: Fit::parse(attributes.get("fit").map(String::as_str).unwrap_or("")),
            max_fps: attributes.get("max-fps").and_then(|fps| fps.parse::<u32>().ok()).filter(|fps| *fps > 0),
        })
    }

    /// What cannot change without starting the pipeline again.
    pub fn needs_restart(&self, other: &Settings) -> bool {
        self.source != other.source || self.muted != other.muted || self.max_fps != other.max_fps
    }
}

/// A rate the player will play at: 0.1 to 8 times, whatever the app asked.
pub fn clamp_rate(rate: f64) -> f64 {
    if !rate.is_finite() {
        return 1.0;
    }
    rate.clamp(0.1, 8.0)
}

/// A URI for a path, or the URI as it is.
fn resolve(source: &str, base: &std::path::Path) -> String {
    if source.contains("://") {
        return source.to_string();
    }
    let path = if source.starts_with('/') { std::path::PathBuf::from(source) } else { base.join(source) };
    format!("file://{}", path.display())
}

/// The size to scale a video of `native` pixels to, for a box of `target`, so that drawing it
/// centred in the box fits it as `fit` says. Both dimensions are even, as video wants.
pub fn scaled_size(native: (u32, u32), target: (u32, u32), fit: Fit) -> (u32, u32) {
    let (native_width, native_height) = (native.0.max(1) as f64, native.1.max(1) as f64);
    let (box_width, box_height) = (target.0.max(2) as f64, target.1.max(2) as f64);
    let (width, height) = match fit {
        Fit::Fill => (box_width, box_height),
        Fit::Cover => {
            let factor = (box_width / native_width).max(box_height / native_height);
            (native_width * factor, native_height * factor)
        }
        Fit::Contain => {
            let factor = (box_width / native_width).min(box_height / native_height);
            (native_width * factor, native_height * factor)
        }
    };
    let even = |length: f64| ((length.round() as u32).max(2) + 1) & !1;
    (even(width), even(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    }

    fn settings(pairs: &[(&str, &str)]) -> Option<Settings> {
        Settings::from_attributes(&attributes(pairs), std::path::Path::new("/app"))
    }

    #[test]
    fn a_video_without_a_source_plays_nothing() {
        assert_eq!(settings(&[]), None);
        assert_eq!(settings(&[("src", "")]), None);
    }

    #[test]
    fn it_loops_and_is_muted_unless_told_otherwise() {
        let found = settings(&[("src", "/a/b.mp4")]).unwrap();
        assert!(found.looping && found.muted && !found.paused);
        assert_eq!((found.rate, found.fit, found.max_fps), (1.0, Fit::Contain, None));
        let found = settings(&[("src", "/a/b.mp4"), ("loop", "false"), ("muted", "false"), ("paused", "")]).unwrap();
        assert!(!found.looping && !found.muted && found.paused);
    }

    #[test]
    fn paths_become_uris_and_relative_ones_start_in_the_app() {
        assert_eq!(settings(&[("src", "/a/b.mp4")]).unwrap().source, "file:///a/b.mp4");
        assert_eq!(settings(&[("src", "clips/b.mp4")]).unwrap().source, "file:///app/clips/b.mp4");
        assert_eq!(settings(&[("src", "https://x.test/b.webm")]).unwrap().source, "https://x.test/b.webm");
    }

    #[test]
    fn the_rate_stays_within_what_playback_handles() {
        assert_eq!(clamp_rate(0.0), 0.1);
        assert_eq!(clamp_rate(100.0), 8.0);
        assert_eq!(clamp_rate(f64::NAN), 1.0);
        assert_eq!(clamp_rate(2.5), 2.5);
    }

    #[test]
    fn only_the_source_sound_and_frame_limit_need_a_new_pipeline() {
        let base = settings(&[("src", "/a.mp4")]).unwrap();
        assert!(
            !base.needs_restart(
                &settings(&[("src", "/a.mp4"), ("rate", "2"), ("paused", ""), ("fit", "cover")]).unwrap()
            )
        );
        assert!(base.needs_restart(&settings(&[("src", "/b.mp4")]).unwrap()));
        assert!(base.needs_restart(&settings(&[("src", "/a.mp4"), ("muted", "false")]).unwrap()));
        assert!(base.needs_restart(&settings(&[("src", "/a.mp4"), ("max-fps", "30")]).unwrap()));
    }

    #[test]
    fn scaling_covers_contains_or_fills_the_box() {
        // 4K video into a 1920 by 1200 box.
        assert_eq!(scaled_size((3840, 2160), (1920, 1200), Fit::Cover), (2134, 1200));
        assert_eq!(scaled_size((3840, 2160), (1920, 1200), Fit::Contain), (1920, 1080));
        assert_eq!(scaled_size((3840, 2160), (1920, 1200), Fit::Fill), (1920, 1200));
    }

    #[test]
    fn scaled_sizes_are_even_and_never_empty() {
        let (width, height) = scaled_size((1001, 999), (333, 333), Fit::Contain);
        assert!(width % 2 == 0 && height % 2 == 0);
        assert_eq!(scaled_size((0, 0), (0, 0), Fit::Cover), (2, 2));
    }
}
