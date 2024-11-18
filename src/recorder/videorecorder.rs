use log::{info, warn, error};
use gst::prelude::*;
use gstreamer::{element_error, Element};
use gstreamer::glib::ControlFlow;
use gstreamer_app::{gst, AppSink};
use std::error::Error;
use gstreamer::ffi::GstPipeline;

#[derive(PartialEq, Debug)]
pub enum PipelineError {
    ParseError,
    EncodingError,
}
pub trait Recorder {
    fn start(&mut self) -> Result<bool, PipelineError>;
    fn stop(&mut self) -> Result<bool, PipelineError>;

    fn on_chunk(&mut self, f: fn(msg: &String) -> ());
}


pub struct VideoRecorder {
    pipeline: String,
    gst_pipeline: Option<Element>,
    on_chunk: Option<fn(&String) -> ()>,
    chunk_sec: u64,
    output_dir: String,
    chunk_prefix: String,
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
        let mut binding = self.gst_pipeline.as_ref().expect("Pipeline mangled").downcast_ref::<gst::Bin>().unwrap()
            .by_name("recording-sink")
            .unwrap();

        binding.set_property("location", format!("{}/{}_%05d.ts",&self.output_dir,&self.chunk_prefix));
        binding.set_property("max-size-time", &self.chunk_sec * 1_000_000_000);
        self.gst_pipeline.as_ref().unwrap().set_state(gst::State::Playing).or_else(|e| {
            error!("{e}");
            Err(PipelineError::EncodingError)
        })?;
        Ok(true)
    }

    fn stop(&mut self) -> Result<bool, PipelineError> {
        info!("Stopping pipeline: {}", self.pipeline);
        self.gst_pipeline.as_ref().unwrap().set_state(gst::State::Null).unwrap();
        Ok(true)
    }

    fn on_chunk(&mut self, f: fn(&String) -> ()) {
        self.on_chunk = Some(f);
        info!("Setting on_chunk callback");
    }
}

#[derive(Default)]
pub struct VideoRecorderBuilder {
    pipeline: String,
    chunk_sec: u64,
    on_chunk: Option<fn(&String) -> ()>,
    output_dir: String,
    chunk_prefix: String,
}
impl VideoRecorderBuilder {
    pub fn new() -> VideoRecorderBuilder {
        gst::init().unwrap();
        VideoRecorderBuilder{
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

    pub fn with_chunks_sec(mut self, s: u64) -> VideoRecorderBuilder {
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
    pub fn build(self) -> VideoRecorder {
        VideoRecorder {
            pipeline: self.pipeline,
            on_chunk: self.on_chunk,
            chunk_sec: self.chunk_sec,
            output_dir: self.output_dir,
            gst_pipeline: None,
            chunk_prefix: self.chunk_prefix,
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
