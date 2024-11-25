use std::ops::Deref;
use byte_slice_cast::AsSliceOf;
use gstreamer::{BufferRef};
use gstreamer_app::gst;
use log::info;
use opencv::{
    core::{self, Mat, Vector,hconcat,  Size},
    imgcodecs,
    imgproc,

};
use opencv::boxed_ref::BoxedRef;
use opencv::core::Vec3b;
use opencv::prelude::*;

pub trait FrameHandler {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError>;
    fn collect_frames(&mut self) -> Result<(), gst::FlowError>;
}

pub struct FrameHandlerImpl{
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
        let map = frame.map_readable().map_err(|_| {
            gst::FlowError::Error
        })?;
        let mut rgb = Vec::<u8>::new();
         map.clone_into(rgb.as_mut());
        let mat = unsafe { Mat::new_rows_cols_with_data_unsafe_def(480, 720, Vec3b::opencv_type(), rgb.as_mut_ptr().cast()) }.unwrap();
        self.frames.push(mat.clone());
        if self.frames.len() == 6 {
            self.collect_frames()?;
        }
        Ok(())
    }

    fn collect_frames(&mut self) -> Result<(), gst::FlowError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let sprite = concat_sprites(create_sprites(&self.frames).as_ref());

        imgcodecs::imwrite(format!("sprite_{}.png", self.idx).as_str(), &sprite, &Vector::new()).unwrap();
        let tooltips= concat_sprites(&self.frames);
        imgcodecs::imwrite(format!("tooltips_{}.png", self.idx).as_str(), &tooltips, &Vector::new()).unwrap();
        self.frames.clear();
        self.idx += 1;
        Ok(())
    }
}

fn create_sprite(input :&Mat) -> Mat {
    let new_height = 54;
    let scale_factor = new_height as f64 / input.rows() as f64;
    let new_width = (input.cols() as f64 * scale_factor).round() as i32;
    let mut img = Mat::default();
    opencv::imgproc::resize(&input, &mut img, Size::new(new_width,new_height), 0.0, 0.0, imgproc::INTER_LINEAR).unwrap();
    let center = img.cols() / 2;
    let roi = img.roi(core::Rect::new(center - 2, 0, 4, img.rows())).unwrap();
    roi.clone_pointee()
}

fn create_sprites(input: &Vec<Mat>) -> Vec<Mat>  {
    // Scaling the image to keep the aspect ratio
    let new_height = 54;
    let mut mat_vec : Vec<Mat> = Vec::new();
    let mut img = Mat::default();
    for frame in input.iter() {
        let scale_factor = new_height as f64 / frame.rows() as f64;
        let new_width = (frame.cols() as f64 * scale_factor).round() as i32;
        opencv::imgproc::resize(&frame, &mut img, Size::new(new_width,new_height), 0.0, 0.0, imgproc::INTER_LINEAR).unwrap();
        let center = img.cols() / 2;
        let sprite = img.roi(core::Rect::new(center - 2, 0, 4, img.rows())).unwrap();
        mat_vec.push(sprite.clone_pointee());
    }
    mat_vec
}
macro_rules! bref_from_mat {
    ($val:expr) => {
        $val.roi(core::Rect::new(0, 0, $val.cols(), $val.rows()))
    };
}
fn concat_sprites(input: &Vec<Mat>) -> Mat {
    use opencv::core::CV_8UC3;
    let mut rgb = vec![0u8; 480 * 720 * 3];
    let black_frame= unsafe { Mat::new_rows_cols_with_data_unsafe_def(480, 720, Vec3b::opencv_type(), rgb.as_mut_ptr().cast()) }.unwrap();

    let mut roi_vec = Vector::<BoxedRef<Mat>>::new();
    for mat in input.iter() {
        let ref_mat = bref_from_mat!(mat).unwrap(); //mat.roi(core::Rect::new(0, 0, mat.cols(), mat.rows())).unwrap();
        roi_vec.push(ref_mat);
    }
    for _ in 0..(6 - input.len()) {
        let ref_mat = bref_from_mat!(input.last().unwrap()).unwrap(); //mat.roi(core::Rect::new(0, 0, mat.cols(), mat.rows())).unwrap();
        roi_vec.push(ref_mat);
    }
    let mut result = core::Mat::default();
    hconcat(&roi_vec, &mut result).unwrap();
    result
}

#[cfg(test)]
mod tests {
    use opencv::core::CV_8UC3;
    use super::*;

    #[test]
    fn test_opencv() {
        let mat = Mat::default();
        let img3: Mat = unsafe { Mat::new_nd(&[480, 640], CV_8UC3).unwrap() };
        //Mat::from_bytes::<opencv::core>(&[0u8; 480 * 640 * 3]).unwrap();
        imgcodecs::imwrite("sprite.png", &img3, &Vector::new()).unwrap();
    }
}