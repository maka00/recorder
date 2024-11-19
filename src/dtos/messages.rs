use std::time::Duration;

pub struct ChunkInfo {
    pub chunk: String,
    pub timestamp: String,
    pub duration: Duration
}

impl ChunkInfo {
    pub fn new(chunk: String, timestamp: String, duration: Duration) -> ChunkInfo {
        ChunkInfo { chunk, timestamp, duration }
    }
}