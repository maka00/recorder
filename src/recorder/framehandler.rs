use byte_slice_cast::AsSliceOf;
use gstreamer::BufferRef;
use gstreamer_app::gst;

pub trait FrameHandler {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError>;
    fn collect_frames(&mut self) -> Result<(), gst::FlowError>;
}

pub struct FrameHandlerImpl{
    pub frames: Vec<Vec<[u8;3]>>,
}

impl FrameHandlerImpl {
    pub fn new() -> FrameHandlerImpl {
        FrameHandlerImpl {
            frames: Vec::new(),
        }
    }
}
impl FrameHandler for FrameHandlerImpl {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError> {
        let map = frame.map_readable().map_err(|_| {
            gst::FlowError::Error
        })?;
        let samples = map.as_slice_of::<[u8;3]>().map_err(|_| {
            gst::FlowError::Error
        })?;
        self.frames.push(Vec::from(samples.clone()));
        Ok(())
    }

    fn collect_frames(&mut self) -> Result<(), gst::FlowError> {
        let elements = self.frames.iter().count();
        // print '*' for each element
        print!("{}: ", elements);
        for _ in 0..elements {
            print!("*");
        }
        println!();
        self.frames.clear();
        Ok(())
    }
}