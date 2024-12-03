use serde::{Deserialize, Serialize};
use toml;

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct RecordingConfig {
    pub source_pipeline: String,
    pub recording_pipeline: String,
    pub preview_pipeline: String,
    pub still_pipeline: String,
    pub chunk_size: u32,
    pub output_dir: String,
    pub chunk_prefix: String,
}

pub struct Config {}

impl Config {
    pub fn new() -> Config {
        Config {}
    }

    pub fn read_config(&self, path: &str) -> Result<RecordingConfig, String> {
        let config = std::fs::read_to_string(path);
        match config {
            Ok(config) => Ok(toml::from_str(config.as_str()).unwrap()),
            Err(_) => Err("Error reading config file".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write_config() {
        let config = RecordingConfig {
            recording_pipeline: "test".to_string(),
            source_pipeline: "test".to_string(),
            still_pipeline: "test".to_string(),
            preview_pipeline: "test".to_string(),
            chunk_size: 1024,
            output_dir: "/tmp".to_string(),
            chunk_prefix: "chunk".to_string(),
        };
        let config_str = toml::to_string(&config).unwrap();
        std::fs::write("test.toml", config_str).unwrap();
    }
}
