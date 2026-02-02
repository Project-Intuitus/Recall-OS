mod client;
mod embedding;
mod rate_limiter;

pub use client::*;
pub use embedding::*;
pub use rate_limiter::*;

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub context: Vec<ContextChunk>,
    pub history: Vec<ConversationMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,  // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub id: i64,
    pub content: String,
    pub source: String,
    pub page: Option<i32>,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub content: String,
    pub citations: Vec<CitationRef>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationRef {
    pub chunk_id: i64,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalysisRequest {
    pub video_path: String,
    pub frames: Vec<VideoFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    pub timestamp: f64,
    pub image_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalysisResponse {
    pub segments: Vec<VideoSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub description: String,
    pub topics: Vec<String>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn analyze_video(&self, request: VideoAnalysisRequest) -> Result<VideoAnalysisResponse>;
    async fn transcribe_audio(&self, audio_data: &[u8]) -> Result<String>;
}
