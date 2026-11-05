use thiserror::Error;

#[derive(Error, Debug)]
pub enum SporeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization/Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Audio engine failure: {0}")]
    Audio(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Spotify API error: {0}")]
    Spotify(String),
}

pub type SporeResult<T> = Result<T, SporeError>;
