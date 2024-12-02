use crate::dtos::messages::StillInfo;
use crate::recorder;
use gst::prelude::*;
use gstreamer_app::gst;
use log::{debug, error, info};
use recorder::common::PipelineError;
use std::{thread, time};

const VIDEO_SOURCE: &str = "video-source";
const VIDEO_SINK: &str = "video-sink";

#[allow(dead_code)]
pub trait StillRecorder: Sync + Send {
    fn take_still(&self, name: &str) -> Result<StillInfo, PipelineError>;
}

pub struct StillRecorderImpl {
    device: String,
    prefix: String,
    socket_path: String,
    output_dir: String,
    pipeline_str: String,
}

impl StillRecorder for StillRecorderImpl {
    fn take_still(&self, name: &str) -> Result<StillInfo, PipelineError> {
        debug!("Taking still");
        let still_file = format!("{}/{}-{}.jpg", self.output_dir, self.prefix, name);
        let gst_pipeline = match gst::parse::launch(self.pipeline_str.as_str()) {
            Ok(pipeline) => {
                info!("Pipeline created...");
                Ok(pipeline.downcast::<gst::Pipeline>().unwrap())
            }
            Err(e) => {
                error!("{e}");
                Err(PipelineError::ParseError)
            }
        }
        .expect("Error launching pipeline");
        let source_element = gst_pipeline
            .by_name(VIDEO_SOURCE)
            .expect("Source bin not found");
        let sink_element = gst_pipeline
            .by_name(VIDEO_SINK)
            .expect("Sink bin not found");
        if source_element.has_property("socket-path", None) {
            source_element.set_property("socket-path", &self.socket_path);
        }
        if sink_element.has_property("location", None) {
            sink_element.set_property("location", &still_file);
        }
        let bus = gst_pipeline.bus().expect("Pipeline without bus");
        gst_pipeline.set_state(gst::State::Playing).map_err(|_e| {
            info!("got issues...");
            return PipelineError::EncodingError;
        })?;

        // while pipeline is playing wait
        while let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(1000)) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    info!("End of stream");
                    break;
                }
                gst::MessageView::Error(err) => {
                    error!(
                        "Error from element {:?}: {}",
                        err.src().map(|s| s.path_string()),
                        err.error()
                    );
                    break;
                }
                _ => (),
            }
        }
        thread::sleep(time::Duration::from_secs(1));
        gst_pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Playing` state");

        if let state = gst_pipeline.state(gst::ClockTime::from_mseconds(1000)) {
            debug!("Pipeline state: {:?}", state);
        }

        Ok(StillInfo {
            device: self.device.clone(),
            width: 0,
            height: 0,
            format: "".to_string(),
            still_file,
        })
    }
}

pub struct StillRecorderBuilder {
    device: String,
    prefix: String,
    socket_path: String,
    pipeline_str: String,
    output_dir: String,
}
impl StillRecorderBuilder {
    pub fn new() -> StillRecorderBuilder {
        StillRecorderBuilder {
            device: "video0".to_string(),
            prefix: "still".to_string(),
            socket_path: "/tmp/video0.sock".to_string(),
            pipeline_str: "unixfdsrc name=video-source ! queue ! videoconvert ! jpegenc snapshot=true ! queue ! filesink name=video-sink".to_string(),
            output_dir: "./".to_string(),
        }
    }

    pub fn with_device(mut self, device: &str) -> StillRecorderBuilder {
        self.device = device.to_string();
        self
    }

    pub fn with_still_file_prefix(mut self, still_file_prefix: &str) -> StillRecorderBuilder {
        self.prefix = still_file_prefix.to_string();
        self
    }

    pub fn with_socket_path(mut self, socket_path: &str) -> StillRecorderBuilder {
        self.socket_path = socket_path.to_string();
        self
    }

    pub fn with_pipeline_str(mut self, pipeline_str: &str) -> StillRecorderBuilder {
        self.pipeline_str = pipeline_str.to_string();
        self
    }

    pub fn with_output_dir(mut self, output_dir: &str) -> StillRecorderBuilder {
        self.output_dir = output_dir.to_string();
        self
    }

    pub fn build(&self) -> StillRecorderImpl {
        StillRecorderImpl {
            device: self.device.clone(),
            prefix: self.prefix.clone(),
            socket_path: self.socket_path.clone(),
            output_dir: self.output_dir.clone(),
            pipeline_str: self.pipeline_str.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    #[test]
    fn test_take_still() {
        let _ = gst::init();
        let still_recorder = StillRecorderBuilder::new()
            .with_device("video0")
            .with_pipeline_str("videotestsrc name=video-source ! videoconvert ! jpegenc snapshot=true ! filesink name=video-sink")
            .with_still_file_prefix("still")
            .with_socket_path("/tmp/video.sock")
            .with_output_dir("/tmp")
            .build();
        let still_info = still_recorder.take_still("dummy").unwrap();
        thread::sleep(time::Duration::from_secs(1));
        assert_eq!(still_info.device, "video0");
        assert_eq!(still_info.still_file, "/tmp/still-dummy.jpg");
        remove_file("/tmp/still-dummy.jpg").unwrap();
    }
}
