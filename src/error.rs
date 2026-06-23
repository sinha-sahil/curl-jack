use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireJackError {
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, WireJackError>;
