use crate::recorder;
use gst::prelude::*;
use gstreamer_app::gst;
use log::{debug, error, info};
use recorder::common::PipelineError;

const VIDEO_SOURCE: &str = "video-source";
const VIDEO_SINK: &str = "video-sink";

#[allow(dead_code)]
pub trait Preview: Sync + Send {
    fn start(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError>;
    fn stop(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError>;
    fn prepare_pipeline(&self, cmd: &str) -> Result<Option<gst::Pipeline>, PipelineError>;
    fn get_pipeline(&self) -> String;
}

pub struct PreviewImpl {
    device: String,
    socket_path: String,
    pipeline_str: String,
}

impl Preview for PreviewImpl {
    fn start(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError> {
        debug!("Webrtc preview");

        let source_element = gst_pipeline
            .as_ref()
            .unwrap()
            .by_name(VIDEO_SOURCE)
            .expect("Source bin not found");
        let sink_element = gst_pipeline
            .as_ref()
            .unwrap()
            .by_name(VIDEO_SINK)
            .expect("Sink bin not found");
        if source_element.has_property("socket-path", None) {
            source_element.set_property("socket-path", &self.socket_path);
        }
        if sink_element.has_property("web-server-directory", None) {
            info!("Setting web-server-directory");
        }
        // ToDo: add a bus watcher
        let _bus = gst_pipeline
            .as_ref()
            .unwrap()
            .bus()
            .expect("Pipeline without bus");
        gst_pipeline
            .as_ref()
            .unwrap()
            .set_state(gst::State::Playing)
            .map_err(|_e| {
                info!("got issues...");
                return PipelineError::EncodingError;
            })?;
        if log::log_enabled!(log::Level::Debug) {
            gst_pipeline
                .as_ref()
                .unwrap()
                .debug_to_dot_file(gst::DebugGraphDetails::MEDIA_TYPE, "preview");
        }
        Ok(())
    }

    fn stop(&self, gst_pipeline: &Option<gst::Pipeline>) -> Result<(), PipelineError> {
        // send eos through the pipeline
        match gst_pipeline
            .as_ref()
            .unwrap()
            .send_event(gst::event::Eos::new())
        {
            false => {
                error!("Failed to send eos event");
            }
            true => {
                info!("EOS sent");
            }
        }
        match gst_pipeline.as_ref().unwrap().set_state(gst::State::Null) {
            Err(_) => {
                error!("Failed to stop pipeline");
                Err(PipelineError::EncodingError)
            }
            Ok(_) => {
                info!("Pipeline stopped");
                Ok(())
            }
        }
    }
    fn prepare_pipeline(&self, cmd: &str) -> Result<Option<gst::Pipeline>, PipelineError> {
        match gst::parse::launch(cmd) {
            Ok(pipeline) => {
                info!("Preview pipeline created...");
                Ok(Some(pipeline.downcast::<gst::Pipeline>().unwrap()))
            }
            Err(e) => {
                error!("{e}");
                Err(PipelineError::ParseError)
            }
        }
    }

    fn get_pipeline(&self) -> String {
        self.pipeline_str.clone()
    }
}

pub struct PreviewBuilder {
    device: String,
    socket_path: String,
    pipeline_str: String,
}
impl PreviewBuilder {
    pub fn new() -> PreviewBuilder {
        PreviewBuilder{
            device: "video0".to_string(),
            socket_path: "/tmp/video0.sock".to_string(),
            pipeline_str: "unixfdsrc name=video-source ! queue ! videoconvert ! webrtcsink  run-signalling-server=true run-web-server=true web-server-directory=client name=video-sink".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_device(mut self, device: &str) -> PreviewBuilder {
        self.device = device.to_string();
        self
    }

    #[allow(dead_code)]
    pub fn with_socket_path(mut self, socket_path: &str) -> PreviewBuilder {
        self.socket_path = socket_path.to_string();
        self
    }

    pub fn with_pipeline_str(mut self, pipeline_str: &str) -> PreviewBuilder {
        self.pipeline_str = pipeline_str.to_string();
        self
    }

    pub fn build(&self) -> PreviewImpl {
        PreviewImpl {
            device: self.device.clone(),
            socket_path: self.socket_path.clone(),
            pipeline_str: self.pipeline_str.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};
    #[test]
    fn test_take_still() {
        let _ = gst::init();
        let preview = PreviewBuilder::new()
            .with_device("video0")
            .with_pipeline_str(
                "videotestsrc name=video-source ! videoconvert !  fakesink name=video-sink",
            )
            .with_socket_path("/tmp/video.sock")
            .build();
        if let Ok(pipeline) = preview.prepare_pipeline(preview.get_pipeline().as_str()) {
            let still_info = preview.start(&pipeline);
            assert!(still_info.is_ok());
        } else {
            assert!(false);
        }
        thread::sleep(time::Duration::from_secs(1));
    }
}
