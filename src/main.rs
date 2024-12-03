mod dtos;
mod recorder;
mod utils;

use crate::dtos::messages::{RecordingInfo, StillInfo, TIMESTAMP_FORMAT};
use crate::recorder::videocontroller::{VideoController, VideoControllerImpl};
use crate::utils::config::RecordingConfig;
use crate::ApiError::StillError;
use crate::ApiResponse::{Still, VideoRecording, VideoSource};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use env_logger::Env;
use log::{error, info};
use std::io::Write;
use std::sync::{Arc, Mutex};

enum ApiResponse {
    Still(StillInfo),
    VideoRecording(RecordingInfo),
    VideoSource,
}
impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Still(still) => (StatusCode::OK, Json(still)).into_response(),
            Self::VideoRecording(info) => (StatusCode::OK, Json(info)).into_response(),
            Self::VideoSource => (StatusCode::OK).into_response(),
        }
    }
}

enum ApiError {
    StillError,
    RecordingError,
    SourceError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::StillError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Error taking still"),
            )
                .into_response(),
            Self::RecordingError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Error during recording"),
            )
                .into_response(),
            Self::SourceError => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json("Error in source")).into_response()
            }
        }
    }
}

async fn root() -> Json<&'static str> {
    Json("Hello, World!")
}

async fn start(State(state): State<Arc<Mutex<AppState>>>) -> Result<ApiResponse, ApiError> {
    info!("Starting recording");
    state
        .lock()
        .unwrap()
        .controller
        .start("video0")
        .map_or_else(|_| Err(ApiError::SourceError), |_| Ok(VideoSource))
}

async fn start_recording(
    State(state): State<Arc<Mutex<AppState>>>,
) -> Result<ApiResponse, ApiError> {
    info!("Starting recording");
    state
        .lock()
        .unwrap()
        .controller
        .start_recording()
        .map_or_else(|_| Err(ApiError::RecordingError), |r| Ok(VideoRecording(r)))
}

async fn stop(State(state): State<Arc<Mutex<AppState>>>) -> Result<ApiResponse, ApiError> {
    info!("Stopping recording");
    state
        .lock()
        .unwrap()
        .controller
        .stop("video0")
        .map_or_else(|_| Err(ApiError::RecordingError), |_| Ok(VideoSource))
}

async fn stop_recording(
    State(state): State<Arc<Mutex<AppState>>>,
) -> Result<ApiResponse, ApiError> {
    info!("Stopping recording");
    state
        .lock()
        .unwrap()
        .controller
        .stop_recording()
        .map_or_else(|_| Err(ApiError::RecordingError), |_| Ok(VideoSource))
}

async fn take_still(State(state): State<Arc<Mutex<AppState>>>) -> Result<ApiResponse, ApiError> {
    info!("Stopping recording");
    // a string holding the current time
    let time = Local::now().format(TIMESTAMP_FORMAT).to_string();
    let still_info = state
        .lock()
        .unwrap()
        .controller
        .take_still("video0", time.as_str())
        .map_or_else(|_| Err(StillError), |still| Ok(Still(still)));
    still_info
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
        let recording_path = std::env::var("RECORDING_PATH").unwrap_or(conf.output_dir.to_string());
        info!("Recording path: {}", recording_path);
        let conf = RecordingConfig {
            output_dir: recording_path,
            ..conf
        };
        info!("Config: {:?}", conf);
        let shared_state = Arc::new(Mutex::new(AppState {
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
                recorder::stillrecorder::StillRecorderBuilder::new()
                    .with_output_dir(conf.output_dir.as_str())
                    .with_pipeline_str(conf.still_pipeline.as_str())
                    .build(),
            ),
        }));

        // build our application with a route
        let app = Router::new()
            // `GET /` goes to `root`
            .route("/", get(root))
            .route("/start", post(start))
            .route("/still", post(take_still))
            .route("/recording/start", post(start_recording))
            .route("/recording/stop", post(stop_recording))
            .route("/stop", post(stop))
            .with_state(shared_state);

        // run our app with hyper, listening globally on port 3000
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        error!("Failed to read config file");
    }
}
