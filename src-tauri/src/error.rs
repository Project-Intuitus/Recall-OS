use thiserror::Error;

#[derive(Error, Debug)]
pub enum RecallError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("PDF extraction error: {0}")]
    PdfExtract(String),

    #[error("LLM API error: {0}")]
    LlmApi(String),

    #[error("Rate limit exceeded: retry after {0} seconds")]
    RateLimit(u64),

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Ingestion error: {0}")]
    Ingestion(String),

    #[error("FFmpeg error: {0}")]
    FFmpeg(String),

    #[error("OCR error: {0}")]
    Ocr(String),

    #[error("Vector search error: {0}")]
    VectorSearch(String),

    #[error("Extension loading error: {0}")]
    ExtensionLoad(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Screen capture error: {0}")]
    Capture(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Tauri error: {0}")]
    Tauri(String),

    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for RecallError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<anyhow::Error> for RecallError {
    fn from(err: anyhow::Error) -> Self {
        RecallError::Other(err.to_string())
    }
}

impl From<tauri::Error> for RecallError {
    fn from(err: tauri::Error) -> Self {
        RecallError::Tauri(err.to_string())
    }
}

impl From<notify::Error> for RecallError {
    fn from(err: notify::Error) -> Self {
        RecallError::Other(format!("File watcher error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, RecallError>;
