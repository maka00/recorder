use crate::dtos::messages::ChunkInfo;
use crate::{dtos, recorder};
use dtos::messages::VideoSourceInfo;
use futures::StreamExt;
use gst::prelude::*;
use gstreamer::ffi::{GstFraction, GstPipeline};
use gstreamer::glib::ControlFlow;
use gstreamer::{element_error, Caps, Element};
use gstreamer_app::{gst, AppSink};
use log::{debug, error, info, warn};
use recorder::common::PipelineError;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;

const VIDEO_SOURCE: &str = "video-source";
const VIDEO_SINK: &str = "video-sink";

pub trait Source {
    // Scan for video sources
    // returns: a list of video sources (e.g. /dev/video0, /dev/video1)
    fn scan(&self) -> Result<Vec<String>, PipelineError>;

    // Start the video source
    // device: the device to start
    // returns: the video source info
    fn start(&mut self, device: &str) -> Result<VideoSourceInfo, PipelineError>;

    // Stop the video source
    // device: the device to start
    fn stop(&mut self, device: String) -> Result<(), PipelineError>;
}

pub struct VideoSource {
    fd_dir: String,
    pipeline_str: String,
    gst_pipeline: Option<Element>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
    runtime: Runtime,
    device: String,
}

impl Source for VideoSource {
    fn scan(&self) -> Result<Vec<String>, PipelineError> {
        info!("Scanning for video sources");
        Ok(std::fs::read_dir("/dev")
            .unwrap()
            .filter_map(|entry| entry.ok().and_then(|e| e.file_name().into_string().ok()))
            .filter(|entry| entry.starts_with("video"))
            .collect::<Vec<String>>())
    }

    fn start(&mut self, device: &str) -> Result<VideoSourceInfo, PipelineError> {
        info!("Starting video source: {}", device);
        self.device = device.to_string();
        println!("Starting source pipeline: {}", &self.pipeline_str);
        match gst::parse::launch(&self.pipeline_str) {
            Ok(pipeline) => {
                self.gst_pipeline = Some(pipeline);
            }
            Err(e) => {
                error!("{e}");
                return Err(PipelineError::ParseError);
            }
        }
        let pipeline_bin = self
            .gst_pipeline
            .as_ref()
            .expect("Pipeline mangled")
            .downcast_ref::<gst::Bin>()
            .unwrap();
        let sink = pipeline_bin
            .by_name(VIDEO_SINK)
            .expect("Unable to get video sink");
        sink.set_property("socket-path", format!("/tmp/{}.sock", &device));

        let source = pipeline_bin
            .by_name(VIDEO_SOURCE)
            .expect("Unable to get video source");
        if source.has_property("device", None) {
            source.set_property("device", format!("/{}/{}", &self.fd_dir, &device));
        }

        let bus = self
            .gst_pipeline
            .as_ref()
            .expect("unable to get pipeline for bus")
            .bus()
            .expect("unable to get bus");

        self.gst_pipeline
            .as_ref()
            .unwrap()
            .set_state(gst::State::Playing)
            .or_else(|e| {
                error!("{e}");
                Err(PipelineError::ParseError)
            })?;
        // wait till state changed
        while self.gst_pipeline.as_ref().unwrap().current_state() == gst::State::Null {
            std::thread::sleep(Duration::from_millis(100));
        }

        (self.tx, self.rx) = mpsc::channel::<String>();

        let tt = self.tx.clone();
        self.runtime.spawn(async {
            message_loop(bus, tt).await;
        });
        info!("Pipeline started");
        // query the video source for info (width, height, framerate, format)
        // wait till caps are available
        while sink.static_pad("sink").unwrap().current_caps().is_none() {
            std::thread::sleep(Duration::from_millis(100));
        }
        self.get_video_info(
            sink.static_pad("sink")
                .unwrap()
                .current_caps()
                .unwrap()
                .to_owned(),
        )
        .ok_or(PipelineError::ParseError)
    }

    fn stop(&mut self, device: String) -> Result<(), PipelineError> {
        info!("Stopping video source: {}", device);
        self.gst_pipeline
            .as_ref()
            .unwrap()
            .send_event(gst::event::Eos::new());
        info!("Got: {}", self.rx.recv().unwrap());
        self.gst_pipeline
            .as_ref()
            .unwrap()
            .set_state(gst::State::Null)
            .or_else(|e| {
                error!("{e}");
                Err(PipelineError::EncodingError)
            })?;

        Ok(())
    }
}

impl VideoSource {
    fn get_video_info(&self, caps: Caps) -> Option<VideoSourceInfo> {
        debug!("Caps: {:?}", caps.structure(0).unwrap().name());
        let format = caps.structure(0).unwrap().get::<&str>("format").unwrap();
        let width = caps.structure(0).unwrap().get::<i32>("width").unwrap();
        let height = caps.structure(0).unwrap().get::<i32>("height").unwrap();
        Some(VideoSourceInfo {
            device: self.device.to_string(),
            width: u32::try_from(width).unwrap_or_default(),
            height: u32::try_from(height).unwrap_or_default(),
            format: format.to_string(),
        })
    }
}

async fn message_loop(bus: gst::Bus, tx: mpsc::Sender<String>) {
    let mut messages = bus.stream();

    while let Some(msg) = messages.next().await {
        use gst::MessageView;

        // Determine whether we want to quit: on EOS or error message
        // we quit, otherwise simply continue.
        match msg.view() {
            MessageView::Eos(..) => {
                info!("EOS");
                tx.send("eos".to_string()).unwrap();
                break;
            }
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            _ => (),
        };
    }
}

pub struct VideoSourceBuilder {
    fd_dir: String,
    pipeline_str: String,
}

impl VideoSourceBuilder {
    pub fn new() -> VideoSourceBuilder {
        gst::init().unwrap();
        VideoSourceBuilder {
            fd_dir: "/dev".to_string(),
            pipeline_str:
                "v4l2src name=video-source device=/dev/video0 ! unixfdsink name=video-sink"
                    .to_string(),
        }
    }
    pub fn with_fd_dir(mut self, fd_dir: String) -> VideoSourceBuilder {
        self.fd_dir = fd_dir;
        self
    }

    pub fn with_pipeline(mut self, pipeline: &str) -> VideoSourceBuilder {
        self.pipeline_str = pipeline.to_string();
        self
    }
    pub fn build(&self) -> VideoSource {
        VideoSource {
            fd_dir: self.fd_dir.to_string(),
            pipeline_str: self.pipeline_str.to_string(),
            gst_pipeline: None,
            tx: mpsc::channel::<String>().0,
            rx: mpsc::channel::<String>().1,
            runtime: Runtime::new().unwrap(),
            device: "".to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_test() {
        let _ = env_logger::try_init();
        let mut source = VideoSourceBuilder::new()
            .with_pipeline(
                format!("videotestsrc name={VIDEO_SOURCE} ! unixfdsink name={VIDEO_SINK}").as_str(),
            )
            .build();
        let res = source.start("video0");
        assert_eq!(res.is_ok(), true);
        let res = source.stop("video0".to_string());
        assert_eq!(res.is_ok(), true);
    }
}
