use thiserror::Error;

#[derive(Error, Debug)]
pub enum VocabError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Failed to parse server payload: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Failed to encode payload parameters: {0}")]
    UrlEncoding(#[from] serde_urlencoded::ser::Error),

    #[error("Failed to decode base64 template data: {0}")]
    DecodeError(String),

    #[error("VocabTrainer API Error ({error_type}): {message}")]
    ApiError { error_type: String, message: String },

    #[error("Unexpected response status from host: {0}")]
    BadStatus(reqwest::StatusCode),
}
