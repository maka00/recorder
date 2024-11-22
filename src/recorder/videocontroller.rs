use crate::dtos::messages::ChunkInfo;
use crate::{dtos, recorder};
use dtos::messages::VideoSourceInfo;
use recorder::common::PipelineError;
use recorder::videorecorder::Recorder;
use recorder::videosource::Source;
pub trait VideoController {
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

    // Start recording
    fn start_recording(&mut self, device: &str) -> Result<(), PipelineError>;

    // Stop recording
    fn stop_recording(&mut self, device: &str) -> Result<(), PipelineError>;

    // Take still
    fn take_still(&mut self, device: &str, still_file: &str) -> Result<(), PipelineError>;
}

pub struct VideoControllerImpl {
    recorder: Box<dyn Recorder>,
    source: Box<dyn Source>,
}

impl VideoController for VideoControllerImpl {
    fn scan(&self) -> Result<Vec<String>, PipelineError> {
        self.source.scan()
    }

    fn start(&mut self, device: &str) -> Result<VideoSourceInfo, PipelineError> {
        self.source.start(device)
    }

    fn stop(&mut self, device: String) -> Result<(), PipelineError> {
        self.source.stop(device)
    }

    fn start_recording(&mut self, device: &str) -> Result<(), PipelineError> {
        if let res = self.recorder.start() {
            Ok(())
        } else {
            Err(PipelineError::EncodingError)
        }
    }

    fn stop_recording(&mut self, device: &str) -> Result<(), PipelineError> {
        if let res = self.recorder.stop() {
            Ok(())
        } else {
            Err(PipelineError::EncodingError)
        }
    }

    fn take_still(&mut self, device: &str, still_file: &str) -> Result<(), PipelineError> {
        unimplemented!()
    }
}

impl VideoControllerImpl {
    pub fn new(
        source: impl Source + 'static,
        recorder: impl Recorder + 'static,
    ) -> VideoControllerImpl {
        VideoControllerImpl {
            source: Box::new(source),
            recorder: Box::new(recorder),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::recorder::videorecorder::VideoRecorder;
    use crate::recorder::videorecorder::VideoRecorderBuilder;
    use crate::recorder::videosource::{VideoSource, VideoSourceBuilder};

    #[test]
    fn test_video_controller() {
        let source = VideoSourceBuilder::new()
            .with_fd_dir("/dev".to_string())
            .with_pipeline("videotestsrc name=video-source ! unixfdsink name=video-sink")
            .build();
        let recorder = VideoRecorderBuilder::new()
            .with_pipeline(
                "unixfdsrc name=video-source ! videoconvert ! fakesink name=recording-sink"
                    .to_string(),
            )
            .with_socket_path("/tmp/video0.sock".to_string())
            .build();
        let mut controller = VideoControllerImpl::new(source, recorder);
        let res = controller.start("video0");
        assert_eq!(res.is_ok(), true);
        let res = controller.start_recording("video0");
        assert_eq!(res.is_ok(), true);
        let res = controller.stop_recording("video0");
        assert_eq!(res.is_ok(), true);
        let res = controller.stop("video0".to_string());
        assert_eq!(res.is_ok(), true);
    }
}
