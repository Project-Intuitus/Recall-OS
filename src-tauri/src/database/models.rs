use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub file_type: FileType,
    pub file_size: i64,
    pub file_hash: String,
    pub mime_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ingested_at: Option<DateTime<Utc>>,
    pub status: DocumentStatus,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Pdf,
    Text,
    Markdown,
    Video,
    Audio,
    Image,
    Screenshot,
    Unknown,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => Self::Pdf,
            "txt" | "text" => Self::Text,
            "md" | "markdown" => Self::Markdown,
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::Video,
            "mp3" | "wav" | "flac" | "m4a" | "ogg" => Self::Audio,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => Self::Image,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Image => "image",
            Self::Screenshot => "screenshot",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for FileType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdf" => Ok(Self::Pdf),
            "text" => Ok(Self::Text),
            "markdown" => Ok(Self::Markdown),
            "video" => Ok(Self::Video),
            "audio" => Ok(Self::Audio),
            "image" => Ok(Self::Image),
            "screenshot" => Ok(Self::Screenshot),
            _ => Ok(Self::Unknown),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for DocumentStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Ok(Self::Pending),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: i64,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: i32,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub page_number: Option<i32>,
    pub timestamp_start: Option<f64>,
    pub timestamp_end: Option<f64>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkWithScore {
    pub chunk: Chunk,
    pub score: f64,
    pub search_type: SearchType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    Vector,
    Fts,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub citations: Vec<Citation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub chunk_id: i64,
    pub document_id: String,
    pub document_title: String,
    pub content_snippet: String,
    pub page_number: Option<i32>,
    pub timestamp: Option<f64>,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStats {
    pub total_documents: i64,
    pub completed_documents: i64,
    pub failed_documents: i64,
    pub pending_documents: i64,
    pub processing_documents: i64,
    pub total_chunks: i64,
    pub total_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    pub document_id: String,
    pub file_path: String,
    pub stage: IngestionStage,
    pub progress: f64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionStage {
    Queued,
    Extracting,
    Chunking,
    Embedding,
    Indexing,
    Completed,
    Failed,
}
