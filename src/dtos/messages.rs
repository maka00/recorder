pub struct ChunkInfo {
    pub chunk: String,
    pub timestamp: String,
}

impl ChunkInfo {
    pub fn new(chunk: String, timestamp: String) -> ChunkInfo {
        ChunkInfo { chunk, timestamp }
    }

    pub fn new_now(chunk: String) -> ChunkInfo {
        ChunkInfo { chunk, timestamp: chrono::Local::now().to_string() }
    }
}