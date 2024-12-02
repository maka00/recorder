use crate::{dtos, recorder};
use dtos::messages::VideoSourceInfo;
use gstreamer::Pipeline;
use recorder::common::PipelineError;
use recorder::videorecorder::Recorder;
use recorder::videosource::Source;
use crate::dtos::messages::StillInfo;
use crate::recorder::stillrecorder::{StillRecorder, StillRecorderBuilder};

#[allow(dead_code)]
pub trait VideoController: Sync + Send {
    // Scan for video sources
    // returns: a list of video sources (e.g. /dev/video0, /dev/video1)
    fn scan(&self) -> Result<Vec<String>, PipelineError>;

    // Start the video source
    // device: the device to start
    // returns: the video source info
    fn start(&self, device: &str) -> Result<VideoSourceInfo, PipelineError>;

    // Stop the video source
    // device: the device to start
    fn stop(&self, device: &str) -> Result<(), PipelineError>;

    // Start recording
    fn start_recording(&mut self) -> Result<(), PipelineError>;

    // Stop recording
    fn stop_recording(&self) -> Result<(), PipelineError>;

    // Take still
    fn take_still(&self, device: &str, still_file: &str) -> Result<StillInfo, PipelineError>;
}

pub struct VideoControllerImpl {
    recorder: Box<dyn Recorder>,
    still: Box<dyn StillRecorder>,
    source: Box<dyn Source>,
    recording_pipeline: Option<Pipeline>,
}

impl VideoController for VideoControllerImpl {
    fn scan(&self) -> Result<Vec<String>, PipelineError> {
        self.source.scan()
    }

    fn start(&self, device: &str) -> Result<VideoSourceInfo, PipelineError> {
        self.source.start(device)
    }

    fn stop(&self, device: &str) -> Result<(), PipelineError> {
        self.source.stop(device)
    }

    fn start_recording(&mut self) -> Result<(), PipelineError> {
        self.recording_pipeline = self
            .recorder
            .prepare_pipeline(self.recorder.get_pipeline().as_str())
            .expect("unable to prepare pipeline");
        self.recorder.start(&self.recording_pipeline)
    }

    fn stop_recording(&self) -> Result<(), PipelineError> {
        self.recorder.stop(&self.recording_pipeline)
    }

    fn take_still(&self, _: &str, image_name: &str) -> Result<StillInfo, PipelineError> {
        self.still.take_still(image_name)
    }
}

impl VideoControllerImpl {
    pub fn new(
        source: impl Source + 'static,
        recorder: impl Recorder + 'static,
        still: impl StillRecorder + 'static,
    ) -> VideoControllerImpl {
        VideoControllerImpl {
            source: Box::new(source),
            recorder: Box::new(recorder),
            still: Box::new(still),
            recording_pipeline: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::recorder::videorecorder::VideoRecorderBuilder;
    use crate::recorder::videosource::VideoSourceBuilder;

    #[test]
    fn test_video_controller() {
        let source = VideoSourceBuilder::new()
            .with_fd_dir("/dev")
            .with_pipeline("videotestsrc name=video-source ! unixfdsink name=video-sink")
            .build();
        let recorder = VideoRecorderBuilder::new()
            .with_pipeline(
                "unixfdsrc name=video-source ! videoconvert ! fakesink name=recording-sink"
                    .to_string(),
            )
            .with_socket_path("/tmp/video0.sock".to_string())
            .build();
        let still = StillRecorderBuilder::new()
            .with_device("video0")
            .with_pipeline_str(
                &*"videotestsrc name=video-source ! videoconvert ! jpegenc snapshot=true ! filesink name=video-sink"
                    .to_string(),
            )
            .with_still_file_prefix("still")
            .build();
        let mut controller = VideoControllerImpl::new(source, recorder, still);
        let res = controller.start("video0");
        assert_eq!(res.is_ok(), true);
        let res = controller.start_recording();
        assert_eq!(res.is_ok(), true);
        let res = controller.stop_recording();
        assert_eq!(res.is_ok(), true);
        let res = controller.stop("video0");
        assert_eq!(res.is_ok(), true);
    }
}
