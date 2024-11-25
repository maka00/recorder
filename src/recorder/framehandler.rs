use gstreamer::BufferRef;
use gstreamer_app::gst;
use opencv::boxed_ref::BoxedRef;
use opencv::core::Vec3b;
use opencv::prelude::*;
use opencv::{
    core::{self, hconcat, Mat, Size, Vector},
    imgcodecs, imgproc,
};

const SPRITE_WIDTH: i32 = 4;
const SPRITE_HEIGHT: i32 = 54;
const SPRITE_COUNT: usize = 6;
const WIDTH: i32 = 720;
const HEIGHT: i32 = 480;
pub trait FrameHandler {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError>;
    fn collect_frames(&mut self) -> Result<(), gst::FlowError>;
}

pub struct FrameHandlerImpl {
    pub frames: Vec<Mat>,
    pub idx: usize,
}

impl FrameHandlerImpl {
    pub fn new() -> FrameHandlerImpl {
        FrameHandlerImpl {
            frames: Vec::new(),
            idx: 0,
        }
    }
}
impl FrameHandler for FrameHandlerImpl {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError> {
        let map = frame.map_readable().map_err(|_| gst::FlowError::Error)?;
        let mut rgb = Vec::<u8>::new();
        map.clone_into(rgb.as_mut());
        let mat = unsafe {
            Mat::new_rows_cols_with_data_unsafe_def(
                HEIGHT,
                WIDTH,
                Vec3b::opencv_type(),
                rgb.as_mut_ptr().cast(),
            )
        }
        .unwrap();
        self.frames.push(mat.clone());
        if self.frames.len() == SPRITE_COUNT {
            self.collect_frames()?;
        }
        Ok(())
    }

    fn collect_frames(&mut self) -> Result<(), gst::FlowError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let sprite = concat_sprites(create_sprites(&self.frames).as_ref());

        imgcodecs::imwrite(
            format!("sprite_{:05}.png", self.idx).as_str(),
            &sprite,
            &Vector::new(),
        )
        .unwrap();
        let tooltips = concat_sprites(&self.frames);
        imgcodecs::imwrite(
            format!("tooltips_{:05}.png", self.idx).as_str(),
            &tooltips,
            &Vector::new(),
        )
        .unwrap();
        self.frames.clear();
        self.idx += 1;
        Ok(())
    }
}

fn create_sprite(input: &Mat) -> Mat {
    let new_height = SPRITE_HEIGHT;
    let scale_factor = new_height as f64 / input.rows() as f64;
    let new_width = (input.cols() as f64 * scale_factor).round() as i32;
    let mut img = Mat::default();
    imgproc::resize(
        &input,
        &mut img,
        Size::new(new_width, new_height),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )
    .unwrap();
    let center = img.cols() / 2;
    let roi = img
        .roi(core::Rect::new(center - 2, 0, SPRITE_WIDTH, img.rows()))
        .unwrap();
    roi.clone_pointee()
}

fn create_sprites(input: &Vec<Mat>) -> Vec<Mat> {
    let mut mat_vec: Vec<Mat> = Vec::new();
    for frame in input.iter() {
        let sprite = create_sprite(frame);
        mat_vec.push(sprite);
    }
    mat_vec
}
macro_rules! bref_from_mat {
    ($val:expr) => {
        $val.roi(core::Rect::new(0, 0, $val.cols(), $val.rows()))
    };
}
fn concat_sprites(input: &Vec<Mat>) -> Mat {
    let mut roi_vec = Vector::<BoxedRef<Mat>>::new();
    for mat in input.iter() {
        let ref_mat = bref_from_mat!(mat).unwrap();
        roi_vec.push(ref_mat);
    }
    for _ in 0..(6 - input.len()) {
        let ref_mat = bref_from_mat!(input.last().unwrap()).unwrap();
        roi_vec.push(ref_mat);
    }
    let mut result = core::Mat::default();
    hconcat(&roi_vec, &mut result).unwrap();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::CV_8UC3;

    #[test]
    fn test_opencv() {
        let mat = Mat::default();
        let img3: Mat = unsafe { Mat::new_nd(&[480, 640], CV_8UC3).unwrap() };
        imgcodecs::imwrite("sprite.png", &img3, &Vector::new()).unwrap();
    }
}
