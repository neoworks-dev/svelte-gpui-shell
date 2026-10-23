//! The GStreamer pipeline behind a `g-video` (see the parent module).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use gpui::RenderImage;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use serde_json::json;

use super::{Settings, scaled_size};
use crate::io::{Settle, Sink};
use crate::tree::NodeId;

/// One decoded frame, as BGRA with no padding between rows.
struct Frame {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

/// What the streaming threads and the window share.
struct Shared {
    frame: Mutex<Option<Frame>>,
    /// A wake for the window is on its way and has not been answered by a draw yet.
    wake_pending: AtomicBool,
}

/// How a pipeline plays, which the bus thread needs when it loops.
#[derive(Clone, Copy)]
struct Live {
    rate: f64,
    looping: bool,
}

pub struct Player {
    node: NodeId,
    sink: Sink,
    pipeline: gst::Element,
    scale_input: gst::Element,
    caps: gst::Element,
    shared: Arc<Shared>,
    live: Arc<Mutex<Live>>,
    stop: Arc<AtomicBool>,
    bus_thread: Option<JoinHandle<()>>,
    settings: Settings,
    /// The size frames are being scaled to, and the box it was worked out for.
    scaled_to: Option<(u32, u32)>,
    image: Option<Arc<RenderImage>>,
    /// Images that were drawn and are now out of date, for the window to free.
    pub stale: Vec<Arc<RenderImage>>,
}

impl Player {
    /// Starts playing. An error says what the pipeline lacks, such as a plugin.
    pub fn start(node: NodeId, settings: Settings, sink: Sink) -> Result<Player, String> {
        gst::init().map_err(|error| format!("GStreamer does not start: {error}"))?;
        if let Some(path) = settings.source.strip_prefix("file://") {
            if !std::path::Path::new(path).is_file() {
                return Err(format!("the video file does not exist: {path}"));
            }
        }
        let shared = Arc::new(Shared { frame: Mutex::new(None), wake_pending: AtomicBool::new(false) });
        let make = |factory: &str| {
            gst::ElementFactory::make(factory)
                .build()
                .map_err(|_| format!("GStreamer has no {factory:?} element, which video needs"))
        };
        let convert = make("videoconvert")?;
        let scale = make("videoscale")?;
        let caps = make("capsfilter")?;
        let appsink = gst_app::AppSink::builder().max_buffers(2).drop(true).sync(true).build();
        let bin = gst::Bin::new();
        let mut chain: Vec<gst::Element> = Vec::new();
        if let Some(fps) = settings.max_fps {
            let limit = make("videorate")?;
            limit.set_property("drop-only", true);
            limit.set_property("max-rate", fps as i32);
            chain.push(limit);
        }
        chain.extend([convert, scale.clone(), caps.clone(), appsink.clone().upcast()]);
        bin.add_many(chain.iter()).map_err(|error| error.to_string())?;
        gst::Element::link_many(chain.iter()).map_err(|error| error.to_string())?;
        let entry = chain[0].static_pad("sink").ok_or("the video sink has no input")?;
        let ghost = gst::GhostPad::with_target(&entry).map_err(|error| error.to_string())?;
        bin.add_pad(&ghost).map_err(|error| error.to_string())?;
        caps.set_property("caps", bgra_caps(None));

        let live = Arc::new(Mutex::new(Live { rate: settings.rate, looping: settings.looping }));
        appsink.set_callbacks(frame_callbacks(node, shared.clone(), sink.clone()));

        let pipeline = make("playbin")?;
        pipeline.set_property("uri", &settings.source);
        pipeline.set_property("video-sink", &bin);
        // Without audio in the flags nothing decodes it, which is what a muted video wants.
        let flags = if settings.muted { "video+deinterlace" } else { "video+audio+deinterlace" };
        pipeline.set_property_from_str("flags", flags);

        let stop = Arc::new(AtomicBool::new(false));
        let bus = pipeline.bus().ok_or("the pipeline has no bus")?;
        let bus_thread = {
            let (pipeline, live, stop, sink) = (pipeline.clone(), live.clone(), stop.clone(), sink.clone());
            std::thread::Builder::new()
                .name("svelte-gpui-video".into())
                .spawn(move || watch_bus(node, bus, pipeline, live, stop, sink))
                .map_err(|error| error.to_string())?
        };
        let player = Player {
            node,
            sink,
            pipeline,
            scale_input: scale,
            caps,
            shared,
            live,
            stop,
            bus_thread: Some(bus_thread),
            settings: settings.clone(),
            scaled_to: None,
            image: None,
            stale: Vec::new(),
        };
        player.pipeline.set_state(gst::State::Paused).map_err(|error| format!("the video does not open: {error}"))?;
        player.set_playing(!settings.paused);
        Ok(player)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Takes the attributes that can change while it plays. It does not restart; the caller
    /// starts a new player when [`Settings::needs_restart`] says so.
    pub fn follow(&mut self, wanted: &Settings) {
        let before = self.settings.clone();
        if (wanted.rate - before.rate).abs() > f64::EPSILON || wanted.looping != before.looping {
            *self.live.lock().expect("video live") = Live { rate: wanted.rate, looping: wanted.looping };
            let position = self.pipeline.query_position::<gst::ClockTime>().unwrap_or(gst::ClockTime::ZERO);
            self.seek_to(position);
        }
        if wanted.paused != before.paused {
            self.set_playing(!wanted.paused);
        }
        if wanted.fit != before.fit {
            self.scaled_to = None;
        }
        self.settings = wanted.clone();
    }

    fn set_playing(&self, playing: bool) {
        let state = if playing { gst::State::Playing } else { gst::State::Paused };
        if let Err(error) = self.pipeline.set_state(state) {
            self.sink.send(
                self.node as u64,
                Settle::Video,
                json!({ "name": "error", "detail": { "message": error.to_string() } }),
                None,
            );
        }
    }

    /// Seeks to a position at the current rate. A looping video plays as a segment, so it can
    /// start again without a gap when the segment ends.
    fn seek_to(&self, position: gst::ClockTime) {
        let live = *self.live.lock().expect("video live");
        seek(&self.pipeline, live, position);
    }

    /// Scales the frames to what `fit` makes of a box of `pixels`, when that is not what they
    /// are scaled to already. Small changes are left alone, as a box that is animating would
    /// otherwise renegotiate the pipeline every frame.
    fn fit_to(&mut self, pixels: (u32, u32)) {
        let Some(native) = self.native_size() else {
            return;
        };
        let wanted = scaled_size(native, pixels, self.settings.fit);
        if let Some(current) = self.scaled_to {
            if current.0.abs_diff(wanted.0) < 16 && current.1.abs_diff(wanted.1) < 16 {
                return;
            }
        }
        self.scaled_to = Some(wanted);
        self.caps.set_property("caps", bgra_caps(Some(wanted)));
        if self.settings.paused {
            let position = self.pipeline.query_position::<gst::ClockTime>().unwrap_or(gst::ClockTime::ZERO);
            self.seek_to(position);
        }
    }

    /// The video's own size, once the pipeline knows it.
    fn native_size(&self) -> Option<(u32, u32)> {
        let caps = self.scale_input.static_pad("sink")?.current_caps()?;
        let info = gst_video::VideoInfo::from_caps(&caps).ok()?;
        let (par_numerator, par_denominator) = (info.par().numer().max(1) as f64, info.par().denom().max(1) as f64);
        let width = (info.width() as f64 * par_numerator / par_denominator).round() as u32;
        Some((width, info.height()))
    }

    /// The newest frame as an image, and its size in device pixels, for a box of `pixels`. The
    /// image is made again only when a frame arrived since.
    pub fn picture(&mut self, pixels: (u32, u32)) -> Option<(Arc<RenderImage>, (u32, u32))> {
        self.fit_to(pixels);
        self.shared.wake_pending.store(false, Ordering::Relaxed);
        let frame = self.shared.frame.lock().expect("video frame").take();
        if let Some(frame) = frame {
            let buffer = image::RgbaImage::from_raw(frame.width, frame.height, frame.bytes)?;
            let fresh = Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(buffer)]));
            if let Some(old) = self.image.replace(fresh) {
                self.stale.push(old);
            }
        }
        let image = self.image.clone()?;
        let size = image.size(0);
        Some((image, (size.width.0 as u32, size.height.0 as u32)))
    }

    /// Every image this player made, for the window to free once the player is gone.
    pub fn release(&mut self) -> Vec<Arc<RenderImage>> {
        let mut all = std::mem::take(&mut self.stale);
        all.extend(self.image.take());
        all
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.pipeline.set_state(gst::State::Null);
        if let Some(thread) = self.bus_thread.take() {
            let _ = thread.join();
        }
    }
}

fn bgra_caps(size: Option<(u32, u32)>) -> gst::Caps {
    let mut builder = gst::Caps::builder("video/x-raw").field("format", "BGRA");
    if let Some((width, height)) = size {
        builder = builder
            .field("width", width as i32)
            .field("height", height as i32)
            .field("pixel-aspect-ratio", gst::Fraction::new(1, 1));
    }
    builder.build()
}

fn seek(pipeline: &gst::Element, live: Live, position: gst::ClockTime) {
    let mut flags = gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE;
    if live.looping {
        flags |= gst::SeekFlags::SEGMENT;
    }
    let result =
        pipeline.seek(live.rate, flags, gst::SeekType::Set, position, gst::SeekType::None, gst::ClockTime::NONE);
    if let Err(error) = result {
        eprintln!("svelte-gpui: the video does not seek: {error}");
    }
}

/// Copies each new frame out of the pipeline and wakes the window for it.
fn frame_callbacks(node: NodeId, shared: Arc<Shared>, sink: Sink) -> gst_app::AppSinkCallbacks {
    gst_app::AppSinkCallbacks::builder()
        .new_sample(move |appsink| {
            let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
            let caps = sample.caps().ok_or(gst::FlowError::Error)?;
            let info = gst_video::VideoInfo::from_caps(caps).map_err(|_| gst::FlowError::Error)?;
            let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
            let frame =
                gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info).map_err(|_| gst::FlowError::Error)?;
            let copied = copy_tight(
                frame.plane_data(0).map_err(|_| gst::FlowError::Error)?,
                info.stride()[0] as usize,
                info.width(),
                info.height(),
            );
            *shared.frame.lock().expect("video frame") =
                Some(Frame { width: info.width(), height: info.height(), bytes: copied });
            if !shared.wake_pending.swap(true, Ordering::Relaxed) {
                sink.send(node as u64, Settle::Video, json!({ "name": "frame" }), None);
            }
            Ok(gst::FlowSuccess::Ok)
        })
        .build()
}

/// The rows of a BGRA plane without the padding after each.
fn copy_tight(plane: &[u8], stride: usize, width: u32, height: u32) -> Vec<u8> {
    let row = width as usize * 4;
    if stride == row {
        return plane[..row * height as usize].to_vec();
    }
    let mut bytes = Vec::with_capacity(row * height as usize);
    for line in 0..height as usize {
        bytes.extend_from_slice(&plane[line * stride..line * stride + row]);
    }
    bytes
}

/// Follows the pipeline's bus: loops a segment that ended, and tells the app when the video
/// is ready, has ended, or failed.
fn watch_bus(
    node: NodeId,
    bus: gst::Bus,
    pipeline: gst::Element,
    live: Arc<Mutex<Live>>,
    stop: Arc<AtomicBool>,
    sink: Sink,
) {
    let tell = |name: &str, detail: serde_json::Value| {
        sink.send(node as u64, Settle::Video, json!({ "name": name, "detail": detail }), None);
    };
    let mut announced = false;
    let mut failed = false;
    while !stop.load(Ordering::Relaxed) {
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(200)) else {
            continue;
        };
        match message.view() {
            gst::MessageView::AsyncDone(_) if !announced => {
                announced = true;
                // The first seek makes the rest a segment, and plays it at the rate asked for.
                seek(&pipeline, *live.lock().expect("video live"), gst::ClockTime::ZERO);
                let duration = pipeline.query_duration::<gst::ClockTime>().map(|time| time.mseconds()).unwrap_or(0);
                tell("ready", json!({ "duration": duration }));
            }
            gst::MessageView::SegmentDone(_) => {
                let state = *live.lock().expect("video live");
                if state.looping {
                    seek_without_flush(&pipeline, state);
                } else {
                    tell("end", json!({}));
                }
            }
            gst::MessageView::Eos(_) => {
                let state = *live.lock().expect("video live");
                if state.looping {
                    seek(&pipeline, state, gst::ClockTime::ZERO);
                } else {
                    tell("end", json!({}));
                }
            }
            // One failure posts several errors; the first one names the cause.
            gst::MessageView::Error(error) if !failed => {
                failed = true;
                tell("error", json!({ "message": error.error().to_string() }));
            }
            _ => {}
        }
    }
}

/// Starts the next round of a looping segment without flushing, which is what makes the loop
/// gapless.
fn seek_without_flush(pipeline: &gst::Element, live: Live) {
    let result = pipeline.seek(
        live.rate,
        gst::SeekFlags::SEGMENT,
        gst::SeekType::Set,
        gst::ClockTime::ZERO,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    );
    if let Err(error) = result {
        eprintln!("svelte-gpui: the video does not loop: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_lose_their_padding() {
        let plane = [1u8, 2, 3, 4, 9, 9, 5, 6, 7, 8, 9, 9];
        assert_eq!(copy_tight(&plane, 6, 1, 2), vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(copy_tight(&plane[..8], 4, 1, 2), vec![1, 2, 3, 4, 9, 9, 5, 6]);
    }
}
