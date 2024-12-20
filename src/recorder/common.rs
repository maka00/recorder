#[derive(PartialEq, Debug)]
pub enum PipelineError {
    ParseError,
    EncodingError,
    NotRunning,
    AlreadyStarted,
}
