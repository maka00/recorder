use crate::dtos::messages::{RecordingInfo, StillInfo};
use crate::recorder::preview::Preview;
use crate::recorder::stillrecorder::StillRecorder;
use crate::{dtos, recorder};
use chrono::Local;
use dtos::messages::VideoSourceInfo;
use gstreamer::Pipeline;
use log::error;
use recorder::common::PipelineError;
use recorder::videorecorder::Recorder;
use recorder::videosource::Source;
use std::{thread, time};

#[allow(dead_code)]
pub trait VideoController: Sync + Send {
    // Scan for video sources
    // returns: a list of video sources (e.g. /dev/video0, /dev/video1)
    fn scan(&self) -> Result<Vec<String>, PipelineError>;

    // Start the video source
    // device: the device to start
    // returns: the video source info
    fn start(&mut self, device: &str) -> Result<VideoSourceInfo, PipelineError>;

    // Stop the video source
    // device: the device to start
    fn stop(&self, device: &str) -> Result<(), PipelineError>;

    // Start recording
    fn start_recording(&mut self) -> Result<RecordingInfo, PipelineError>;

    // Stop recording
    fn stop_recording(&self) -> Result<(), PipelineError>;

    // Take still
    fn take_still(&self, device: &str, still_file: &str) -> Result<StillInfo, PipelineError>;
}

pub struct VideoControllerImpl {
    recorder: Box<dyn Recorder>,
    still: Box<dyn StillRecorder>,
    source: Box<dyn Source>,
    preview: Box<dyn Preview>,
    recording_pipeline: Option<Pipeline>,
    preview_pipeline: Option<Pipeline>,
}

impl VideoController for VideoControllerImpl {
    fn scan(&self) -> Result<Vec<String>, PipelineError> {
        self.source.scan()
    }

    fn start(&mut self, device: &str) -> Result<VideoSourceInfo, PipelineError> {
        let res = self.source.start(device);
        let preview_pipeline = self
            .preview
            .prepare_pipeline(self.preview.get_pipeline().as_str())
            .map_or_else(
                |_| Err(PipelineError::ParseError),
                |pipeline| {
                    self.preview_pipeline = pipeline;
                    Ok(())
                },
            );
        match preview_pipeline {
            Ok(_) => self
                .preview
                .start(&self.preview_pipeline)
                .map_or_else(|_| Err(PipelineError::EncodingError), |_| res),
            Err(e) => Err(e),
        }
    }

    fn stop(&self, device: &str) -> Result<(), PipelineError> {
        if let Err(e) = self.preview.stop(&self.preview_pipeline) {
            error!("Error stopping preview pipeline: {:?}", e);
        }
        thread::sleep(time::Duration::from_secs(1));
        self.source.stop(device)
    }

    fn start_recording(&mut self) -> Result<RecordingInfo, PipelineError> {
        let timestamp = Local::now();
        let recording_pipeline = self
            .recorder
            .prepare_pipeline(self.recorder.get_pipeline().as_str())
            .map_or_else(|_| Err(PipelineError::ParseError), |pipeline| Ok(pipeline));
        match recording_pipeline {
            Ok(pipeline) => {
                self.recording_pipeline = pipeline;
                self.recorder.start(&self.recording_pipeline, &timestamp)
            }
            Err(e) => Err(e),
        }
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
        preview: impl Preview + 'static,
    ) -> VideoControllerImpl {
        VideoControllerImpl {
            source: Box::new(source),
            recorder: Box::new(recorder),
            still: Box::new(still),
            preview: Box::new(preview),
            recording_pipeline: None,
            preview_pipeline: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::recorder::preview::PreviewBuilder;
    use crate::recorder::stillrecorder::StillRecorderBuilder;
    use crate::recorder::videorecorder::VideoRecorderBuilder;
    use crate::recorder::videosource::VideoSourceBuilder;
    use std::fs::remove_file;

    #[test]
    fn test_video_controller() {
        let _ = remove_file("/tmp/video0.sock");
        let source = VideoSourceBuilder::new()
            .with_fd_dir("/tmp")
            .with_pipeline("videotestsrc name=video-source ! unixfdsink name=video-sink")
            .build();
        let recorder = VideoRecorderBuilder::new()
            .with_pipeline(
                "unixfdsrc name=video-source ! tee name=t \
                t. \
                ! videoconvert ! fakesink name=video-sink \
                t. \
                ! videoconvert ! appsink name=frame-sink"
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
            .with_still_file_postfix("still")
            .build();
        let preview = PreviewBuilder::new()
            .with_device("video0")
            .with_pipeline_str(
                "videotestsrc name=video-source ! videoconvert ! fakesink name=video-sink",
            )
            .with_socket_path("/tmp/video.sock")
            .build();
        let mut controller = VideoControllerImpl::new(source, recorder, still, preview);
        let res = controller.start("video0");
        assert_eq!(res.is_ok(), true);
        let res = controller.start_recording();
        assert_eq!(res.is_ok(), true);
        let res = controller.take_still("video0", "test");
        assert_eq!(res.is_ok(), true);
        let res = controller.stop_recording();
        assert_eq!(res.is_ok(), true);
        let res = controller.stop("video0");
        assert_eq!(res.is_ok(), true);
        let _ = remove_file("test-still.jpg");
    }
}
