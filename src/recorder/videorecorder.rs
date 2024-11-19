use log::{info, warn, error, debug};
use gst::prelude::*;
use gstreamer::{element_error, Element};
use gstreamer::glib::ControlFlow;
use gstreamer_app::{gst, AppSink};
use std::error::Error;
use futures::StreamExt;
use gstreamer::ffi::GstPipeline;
use tokio::runtime::Runtime;
use tokio::time::*;
use std::sync::{mpsc, Mutex};
use crate::dtos::messages::ChunkInfo;

#[derive(PartialEq, Debug)]
pub enum PipelineError {
    ParseError,
    EncodingError,
}
pub trait Recorder {
    fn start(&mut self) -> Result<bool, PipelineError>;
    fn stop(&mut self) -> Result<bool, PipelineError>;

    fn on_chunk(&mut self, f: fn(msg: &ChunkInfo) -> ());
}


pub struct VideoRecorder {
    pipeline: String,
    gst_pipeline: Option<Element>,
    on_chunk: std::sync::Arc<Mutex<Option<fn(&ChunkInfo) -> ()>>>,
    chunk_sec: u32,
    output_dir: String,
    chunk_prefix: String,
    runtime: Runtime,
    tx : mpsc::Sender<String>,
    rx : mpsc::Receiver<String>,
}

impl Recorder for VideoRecorder {
    fn start(&mut self) -> Result<bool, PipelineError> {
        info!("Starting pipeline: {}", self.pipeline);
        match gst::parse::launch(&self.pipeline) {
            Ok(pipeline) => {
                self.gst_pipeline = Some(pipeline);
            }
            Err(e) => {
                error!("{e}");
                return Err(PipelineError::ParseError);
            }
        }
        let binding = self.gst_pipeline.as_ref().expect("Pipeline mangled").downcast_ref::<gst::Bin>().unwrap()
            .by_name("recording-sink")
            .unwrap();

        binding.set_property("location", format!("{}/{}_%05d.ts", &self.output_dir, &self.chunk_prefix));
        binding.set_property("target-duration", &self.chunk_sec);
        binding.set_property("message-forward", true);

        let bus = self.gst_pipeline.as_ref().expect("unable to get pipeline for bus").bus().expect("unable to get bus");

        self.gst_pipeline.as_ref().unwrap().set_state(gst::State::Playing).or_else(|e| {
            error!("{e}");
            Err(PipelineError::EncodingError)
        })?;
        (self.tx, self.rx) = mpsc::channel::<String>();

        let tt = self.tx.clone();
        let callback = self.on_chunk.clone();
        self.runtime.spawn(async {
            message_loop(bus, tt, callback).await;
        });
        info!("Pipeline started");

        Ok(true)
    }

    fn stop(&mut self) -> Result<bool, PipelineError> {
        info!("Stopping pipeline: {}", self.pipeline);
        self.gst_pipeline.as_ref().unwrap().send_event(gst::event::Eos::new());
        info!("Got: {}",self.rx.recv().unwrap());
        self.gst_pipeline.as_ref().unwrap().set_state(gst::State::Null).unwrap();
        Ok(true)
    }

    fn on_chunk(&mut self, f: fn(&ChunkInfo) -> ()) {
        self.on_chunk = std::sync::Arc::new(Mutex::new(Some(f)));
        info!("Setting on_chunk callback");
    }

}
async fn message_loop(bus: gst::Bus, tx: mpsc::Sender<String>, on_chunk: std::sync::Arc<Mutex<Option<fn(&ChunkInfo) -> ()>>>) {
    let mut messages = bus.stream();

    while let Some(msg) = messages.next().await {
        use gst::MessageView;

        // Determine whether we want to quit: on EOS or error message
        // we quit, otherwise simply continue.
        match msg.view() {
            MessageView::Eos(..) => {
                info!("EOS");
                tx.send("eos".to_string()).unwrap();
                break
            },
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            MessageView::Element(e) => {
                if let Some(s) = msg.structure() {
                    match msg.src().unwrap().name().as_str() {
                        "recording-sink" => {
                            let msg_struct = s;
                            if msg.structure().unwrap().name() == "hls-segment-added" {
                                debug!("location: {}", msg_struct.get::<&str>("location").unwrap().to_string());
                                debug!("running-time: {}", msg_struct.get::<u64>("running-time").unwrap().to_string());
                                debug!("duration: {}", msg_struct.get::<u64>("duration").unwrap().to_string());
                                if let Ok(f) = on_chunk.lock() {
                                    if let Some(ff) = f.as_ref() {
                                        let chunk = ChunkInfo::new(msg_struct.get::<&str>("location").unwrap().to_string(),
                                                                   msg_struct.get::<u64>("running-time").unwrap().to_string(),
                                                                   Duration::from_secs(msg_struct.get::<u64>("duration").unwrap()));
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
    pub fn with_chunk_prefix(mut self, s: String) -> VideoRecorderBuilder {
        self.chunk_prefix = s;
        self
    }
    pub fn with_on_chunk(mut self, f: fn(&ChunkInfo) -> ()) -> VideoRecorderBuilder {
        self.on_chunk = Some(f);
        self
    }
    pub fn build(self) -> VideoRecorder {
        VideoRecorder {
            pipeline: self.pipeline,
            on_chunk: std::sync::Arc::new(Mutex::new(self.on_chunk)),
            chunk_sec: self.chunk_sec,
            output_dir: self.output_dir,
            gst_pipeline: None,
            chunk_prefix: self.chunk_prefix,
            runtime: Runtime::new().unwrap(),
            tx: mpsc::channel::<String>().0,
            rx: mpsc::channel::<String>().1,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::recorder;

    #[test]
    fn build_test() {
        let recorder = recorder::videorecorder::VideoRecorderBuilder::new().with_pipeline("test".to_string()).build();
        assert_eq!(recorder.pipeline, "test");
    }
}
