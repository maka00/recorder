mod dtos;
mod recorder;
mod utils;

use crate::recorder::videocontroller::VideoController;
use crate::recorder::videorecorder::Recorder;
use crate::utils::config::RecordingConfig;
use chrono::Local;
use env_logger::Env;
use log::{debug, error, info, warn, Level};
use std::io::Write;
use std::{thread, time::Duration};

fn record(conf: &RecordingConfig) {
    let recorder = recorder::videorecorder::VideoRecorderBuilder::new()
        .with_pipeline(conf.recording_pipeline.to_string())
        .with_chunks_sec(conf.chunk_size)
        .with_output_dir(conf.output_dir.to_string())
        .with_socket_path("/tmp/video0.sock".to_string())
        .with_on_chunk(|chunk| {
            info!(
                "Chunk: {}, timestamp: {}, duration: {}",
                chunk.chunk,
                chunk.timestamp,
                chunk.duration.as_secs()
            );
        })
        .build();
    let source = recorder::videosource::VideoSourceBuilder::new()
        .with_fd_dir("/tmp".to_string())
        .with_pipeline(conf.source_pipeline.as_str())
        .build();
    let mut controller = recorder::videocontroller::VideoControllerImpl::new(source, recorder);

    if let Ok(status) = controller.start("video0") {
        info!("Controller started: {}", status.device);
    } else {
        error!("Failed to start recorder");
    }
    thread::sleep(Duration::from_secs(2));
    if let Ok(_) = controller.start_recording("video0") {
        info!("Recorder started");
    } else {
        error!("Failed to start recorder");
    }

    thread::sleep(Duration::from_secs(10));
    if let Ok(_) = controller.stop_recording("video0") {
        info!("Recorder stopped");
    } else {
        error!("Failed to stop recorder");
    }
    if let Ok(_) = controller.stop("video0".to_string()) {
        info!("Controller stopped");
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
