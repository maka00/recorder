use chrono::{Duration, NaiveTime};
use gstreamer::BufferRef;
use gstreamer_app::gst;
use opencv::boxed_ref::BoxedRef;
use opencv::core::Vec3b;
use opencv::prelude::*;
use opencv::{
    core::{self, hconcat, Mat, Size, Vector},
    imgcodecs, imgproc,
};
use std::fs::OpenOptions;
use std::io::Write;

const SPRITE_WIDTH: i32 = 4;
const SPRITE_HEIGHT: i32 = 54;
const SPRITE_COUNT: usize = 6;
const WIDTH: i32 = 720;
const HEIGHT: i32 = 480;
pub trait FrameHandler {
    fn handle_frame(&mut self, frame: &BufferRef) -> Result<(), gst::FlowError>;
    fn collect_frames(&mut self) -> Result<(), gst::FlowError>;

    fn reset(&mut self);
}

pub struct FrameHandlerImpl {
    pub frames: Vec<Mat>,
    pub idx: usize,
    pub file: std::fs::File,
}

impl FrameHandlerImpl {
    pub fn new() -> FrameHandlerImpl {
        FrameHandlerImpl {
            frames: Vec::new(),
            idx: 0,
            file: OpenOptions::new()
                .create(true)
                .write(true)
                .open("thumbnails.vtt")
                .unwrap(),
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
            format!("sprite_{:05}.jpg", self.idx).as_str(),
            &sprite,
            &Vector::new(),
        )
        .unwrap();
        let tooltips = concat_sprites(&self.frames);
        imgcodecs::imwrite(
            format!("tooltips_{:05}.jpg", self.idx).as_str(),
            &tooltips,
            &Vector::new(),
        )
        .unwrap();
        self.frames.clear();
        // append or create a textfile with the sprite name

        if self.idx == 0 {
            writeln!(self.file, "WEBVTT").unwrap();
            writeln!(self.file, "").unwrap();
        }
        for i in 1..=SPRITE_COUNT {
            writeln!(self.file, "{}", i + self.idx * 6).unwrap();
            let from_sec = (self.idx + i - 1) + self.idx * 6;
            let to_sec = (self.idx + i) + self.idx * 6;
            writeln!(
                self.file,
                "{} --> {}",
                format_seconds(from_sec),
                format_seconds(to_sec)
            )
            .unwrap();
            let x = (i - 1) * WIDTH as usize;
            let y = 0;
            let w = WIDTH as usize;
            let h = HEIGHT as usize;
            writeln!(
                self.file,
                "tooltips_{:05}.jpg#xywh={},{},{},{}",
                self.idx, x, y, w, h
            )
            .unwrap();
            writeln!(self.file, "").unwrap();
        }
        self.idx += 1;
        Ok(())
    }

    fn reset(&mut self) {
        self.frames.clear();
        self.idx = 0;
    }
}
fn format_seconds(seconds: usize) -> String {
    let duration = Duration::seconds(seconds as i64);
    let time =
        NaiveTime::from_num_seconds_from_midnight_opt(duration.num_seconds() as u32 % 86400, 0)
            .unwrap();
    format!("{}", time.format("%H:%M:%S%.3f"))
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
