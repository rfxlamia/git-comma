use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Model unavailable: {0}")]
    ModelUnavailable(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Empty response from API")]
    EmptyResponse,
}

#[derive(Debug, Error)]
pub enum CommitError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git hook failed: {0}")]
    HookFailed(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}
