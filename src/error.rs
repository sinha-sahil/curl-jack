use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireJackError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("no URL provided")]
    MissingUrl,

    #[error("invalid header format: {0}")]
    InvalidHeader(String),

    #[error("shell tokenization error: {0}")]
    Tokenize(String),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, WireJackError>;
