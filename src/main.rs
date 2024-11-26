mod dtos;
mod recorder;
mod utils;

use crate::recorder::videocontroller::{VideoController, VideoControllerImpl};
use crate::utils::config::RecordingConfig;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use env_logger::Env;
use log::{error, info};
use std::io::Write;
use std::sync::Arc;
use std::{thread, time::Duration};

async fn root() -> Json<&'static str> {
    Json("Hello, World!")
}

async fn start(State(state): State<Arc<AppState>>) -> Json<&'static str> {
    info!("Starting recording");
    state
        .controller
        .start("video0")
        .expect("TODO: panic message");
    state
        .controller
        .start_recording()
        .expect("TODO: panic message");
    Json("Hello, World!")
}
async fn stop(State(state): State<Arc<AppState>>) -> Json<&'static str> {
    info!("Stopping recording");
    state
        .controller
        .stop_recording()
        .expect("TODO: panic message");
    // sleep for 200ms to allow the last chunk to be written
    tokio::time::sleep(Duration::from_millis(200)).await;
    state
        .controller
        .stop("video0")
        .expect("TODO: panic message");
    Json("Hello, World!")
}

struct AppState {
    controller: crate::recorder::videocontroller::VideoControllerImpl,
}

#[tokio::main]
async fn main() {
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
        let shared_state = Arc::new(AppState {
            controller: VideoControllerImpl::new(
                recorder::videosource::VideoSourceBuilder::new()
                    .with_fd_dir("/tmp")
                    .with_pipeline(conf.source_pipeline.as_str())
                    .build(),
                recorder::videorecorder::VideoRecorderBuilder::new()
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
                    .build(),
            ),
        });

        // build our application with a route
        let app = Router::new()
            // `GET /` goes to `root`
            .route("/", get(root))
            .route("/start", post(start))
            .route("/stop", post(stop))
            .with_state(shared_state);

        // run our app with hyper, listening globally on port 3000
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        error!("Failed to read config file");
    }
}
