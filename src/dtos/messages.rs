use serde::Serialize;
use std::time::Duration;

pub const TIMESTAMP_FORMAT: &str = "%Y%m%d-%H%M%S";
#[derive(Default)]
pub struct ChunkInfo {
    pub chunk: String,
    pub timestamp: String,
    pub duration: Duration,
}

impl ChunkInfo {
    pub fn new(chunk: String, timestamp: String, duration: Duration) -> ChunkInfo {
        ChunkInfo {
            chunk,
            timestamp,
            duration,
        }
    }
}

#[derive(Default)]
#[allow(dead_code)]
pub struct VideoSourceInfo {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

pub enum RecordingState {
    ChunkCreated,
    EOS,
}

#[derive(Default, Serialize)]
pub struct RecordingInfo {
    pub prefix: String,
}

#[derive(Default, Serialize)]
pub struct StillInfo {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub still_file: String,
}
