use crate::dtos::messages::ChunkInfo;
use crate::recorder;
use crate::recorder::framehandler::{FrameHandler, FrameHandlerImpl};
use futures::StreamExt;
use gst::prelude::*;
use gstreamer::element_error;
use gstreamer_app::{gst, AppSink};
use log::{debug, error, info};
use recorder::common::PipelineError;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use tokio::time::*;

const VIDEO_SOURCE: &str = "video-source";
const VIDEO_SINK: &str = "video-sink";
const FRAME_SINK: &str = "frame-sink";

#[allow(dead_code)]
pub trait Recorder: Sync + Send {
    fn start(&self, pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError>;
    fn stop(&self, pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError>;
    fn prepare_pipeline(&self, cmd: &str) -> Result<Option<gst::Pipeline>, PipelineError>;

    fn get_pipeline(&self) -> String;
}

pub struct VideoRecorder {
    pipeline: String,
    on_chunk: std::sync::Arc<Mutex<Option<fn(&ChunkInfo) -> ()>>>,
    chunk_sec: u32,
    output_dir: String,
    chunk_prefix: String,
    runtime: Runtime,
    socket_path: String,
    fh: std::sync::Arc<Mutex<FrameHandlerImpl>>,
}

impl Recorder for VideoRecorder {
    fn start(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError> {
        info!("Starting recording pipeline: {}", self.pipeline);
        if gst_pipeline.as_ref().unwrap().current_state() == gst::State::Playing {
            return Err(PipelineError::AlreadyStarted);
        }
        let pipeline_bin = gst_pipeline.as_ref().expect("Pipeline mangled");

        let source_binding = pipeline_bin.by_name(VIDEO_SOURCE).unwrap();
        if source_binding.has_property("socket-path", None) {
            source_binding.set_property("socket-path", &self.socket_path);
        }
        debug!("using socket path: {}", self.socket_path);
        let sink_binding = pipeline_bin.by_name(VIDEO_SINK).unwrap();
        let output_location = format!("{}/{}_%05d.ts", &self.output_dir, &self.chunk_prefix);
        let ols = output_location.as_str();
        info!("Output location: {}", ols);
        if sink_binding.has_property("location", None) {
            sink_binding.set_property("location", output_location);
            sink_binding.set_property("target-duration", &self.chunk_sec);
            sink_binding.set_property("message-forward", true);
        }
        let frame_sink_binding = pipeline_bin.by_name(FRAME_SINK).unwrap();
        let dummy = frame_sink_binding.downcast_ref::<AppSink>();
        let frame_sink = dummy.expect("Frame sink is expected to be an appsink!");
        self.fh.lock().unwrap().reset();
        frame_sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(sample_callback(self.fh.clone()))
                .build(),
        );

        let bus = gst_pipeline
            .as_ref()
            .expect("unable to get pipeline for bus")
            .bus()
            .expect("unable to get bus");

        gst_pipeline
            .as_ref()
            .unwrap()
            .set_state(gst::State::Playing)
            .or_else(|e| {
                error!("{e}");
                Err(PipelineError::EncodingError)
            })?;

        let callback = self.on_chunk.clone();
        let frame_handler = self.fh.clone();
        self.runtime.spawn(async {
            message_loop(bus, callback, frame_handler).await;
        });
        info!("Pipeline started");

        Ok(())
    }
    fn stop(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError> {
        info!("Stopping pipeline: {}", self.pipeline);
        if gst_pipeline.as_ref().unwrap().current_state() == gst::State::Null {
            return Err(PipelineError::NotRunning);
        }
        /*
        self.gst_pipeline
            .as_ref()
            .unwrap()
            .send_event(gst::event::Eos::new());
         */
        gst_pipeline
            .as_ref()
            .unwrap()
            .set_state(gst::State::Null)
            .unwrap();
        Ok(())
    }
    fn prepare_pipeline(&self, cmd: &str) -> Result<Option<gst::Pipeline>, PipelineError> {
        match gst::parse::launch(cmd) {
            Ok(pipeline) => {
                info!("Pipeline created...");
                Ok(Some(pipeline.downcast::<gst::Pipeline>().unwrap()))
            }
            Err(e) => {
                error!("{e}");
                Err(PipelineError::ParseError)
            }
        }
    }

    fn get_pipeline(&self) -> String {
        self.pipeline.clone()
    }
}

fn sample_callback(
    fh: std::sync::Arc<Mutex<FrameHandlerImpl>>,
) -> impl Fn(&AppSink) -> Result<gst::FlowSuccess, gst::FlowError> {
    move |app_sink: &AppSink| {
        let sample = app_sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;

        let buffer = sample.buffer().ok_or_else(|| {
            element_error!(
                app_sink,
                gst::ResourceError::Failed,
                ("Failed to get buffer")
            );
            gst::FlowError::Error
        })?;
        fh.lock().unwrap().handle_frame(&buffer).unwrap();
        let _ = buffer.map_readable().map_err(|_| {
            element_error!(
                app_sink,
                gst::ResourceError::Failed,
                ("Failed to map buffer readable")
            );
            gst::FlowError::Error
        })?;

        Ok(gst::FlowSuccess::Ok)
    }
}

async fn message_loop(
    bus: gst::Bus,
    on_chunk: std::sync::Arc<Mutex<Option<fn(&ChunkInfo) -> ()>>>,
    fh: std::sync::Arc<Mutex<FrameHandlerImpl>>,
) {
    let mut messages = bus.stream();

    while let Some(msg) = messages.next().await {
        use gst::MessageView;

        // Determine whether we want to quit: on EOS or error message
        // we quit, otherwise simply continue.
        match msg.view() {
            MessageView::Eos(..) => {
                info!("EOS");
                fh.lock().unwrap().collect_frames().unwrap();
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
            MessageView::Element(_) => {
                if let Some(s) = msg.structure() {
                    match msg.src().unwrap().name().as_str() {
                        VIDEO_SINK => {
                            let msg_struct = s;
                            if msg.structure().unwrap().name() == "hls-segment-added" {
                                debug!(
                                    "location: {}",
                                    msg_struct.get::<&str>("location").unwrap().to_string()
                                );
                                debug!(
                                    "running-time: {}",
                                    msg_struct.get::<u64>("running-time").unwrap().to_string()
                                );
                                debug!(
                                    "duration: {}",
                                    msg_struct.get::<u64>("duration").unwrap().to_string()
                                );
                                if let Ok(f) = on_chunk.lock() {
                                    if let Some(ff) = f.as_ref() {
                                        let chunk = ChunkInfo::new(
                                            msg_struct.get::<&str>("location").unwrap().to_string(),
                                            msg_struct
                                                .get::<u64>("running-time")
                                                .unwrap()
                                                .to_string(),
                                            Duration::from_secs(
                                                msg_struct.get::<u64>("duration").unwrap(),
                                            ),
                                        );
                                        ff(&chunk);
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        };
    }
}
#[derive(Default)]
pub struct VideoRecorderBuilder {
    pipeline: String,
    chunk_sec: u32,
    on_chunk: Option<fn(&ChunkInfo) -> ()>,
    output_dir: String,
    chunk_prefix: String,
    socket_path: String,
}
impl VideoRecorderBuilder {
    pub fn new() -> VideoRecorderBuilder {
        gst::init().unwrap();
        VideoRecorderBuilder {
            pipeline: "".to_string(),
            chunk_sec: 6,
            on_chunk: None,
            output_dir: ".".to_string(),
            chunk_prefix: "chunk".to_string(),
            socket_path: "/tmp/video.sock".to_string(),
        }
    }

    pub fn with_pipeline(mut self, s: String) -> VideoRecorderBuilder {
        self.pipeline = s;
        self
    }

    pub fn with_chunks_sec(mut self, s: u32) -> VideoRecorderBuilder {
        self.chunk_sec = s;
        self
    }

    pub fn with_output_dir(mut self, s: String) -> VideoRecorderBuilder {
        self.output_dir = s;
        self
    }
    #[allow(dead_code)]
    pub fn with_chunk_prefix(mut self, s: String) -> VideoRecorderBuilder {
        self.chunk_prefix = s;
        self
    }
    pub fn with_on_chunk(mut self, f: fn(&ChunkInfo) -> ()) -> VideoRecorderBuilder {
        self.on_chunk = Some(f);
        self
    }
    pub fn with_socket_path(mut self, s: String) -> VideoRecorderBuilder {
        self.socket_path = s;
        self
    }

    pub fn build(self) -> VideoRecorder {
        VideoRecorder {
            pipeline: self.pipeline.clone(),
            on_chunk: std::sync::Arc::new(Mutex::new(self.on_chunk)),
            chunk_sec: self.chunk_sec,
            output_dir: self.output_dir.clone(),
            chunk_prefix: self.chunk_prefix,
            socket_path: self.socket_path,
            runtime: Runtime::new().unwrap(),
            fh: std::sync::Arc::new(Mutex::new(FrameHandlerImpl::new(self.output_dir))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_test() {
        let recorder = VideoRecorderBuilder::new()
            .with_pipeline("test".to_string())
            .build();
        assert_eq!(recorder.pipeline, "test");
    }
}
