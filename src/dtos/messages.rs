use std::time::Duration;

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
pub struct VideoSourceInfo {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}
