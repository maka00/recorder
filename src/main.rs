mod recorder;
mod dtos;
mod utils;

use log::{debug, info, warn, error, Level};
use env_logger::Env;
use chrono::Local;
use std::io::Write;
use crate::recorder::videorecorder::Recorder;
use crate::utils::config::RecordingConfig;
use std::{thread, time::Duration};

fn record(conf: &RecordingConfig) {
    let mut recorder = recorder::videorecorder::VideoRecorderBuilder::new()
        .with_pipeline(conf.pipeline.to_string())
        .with_chunks_sec(conf.chunk_size)
        .with_output_dir(conf.output_dir.to_string())
        .with_on_chunk(|chunk| {
            info!("Chunk: {}, timestamp: {}, duration: {}", chunk.chunk, chunk.timestamp, chunk.duration.as_secs());
        })
        .build();

    if let Ok(status) = recorder.start() {
        info!("Recorder started: {}", status);
    } else {
        error!("Failed to start recorder");
    }
    thread::sleep(Duration::from_secs(10));
    if let Ok(status) = recorder.stop() {
        info!("Recorder stopped: {}", status);
    } else {
        error!("Failed to stop recorder");
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}:{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
    info!("Starting video recorder");
    if let Ok(conf) = utils::config::Config::new().read_config("config.toml") {
        info!("Config: {:?}", conf);
        record(&conf)
    } else {
        error!("Failed to read config file");
    }
}
