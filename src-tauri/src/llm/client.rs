use super::{
    EmbeddingClient, GenerateRequest, GenerateResponse, LlmProvider, RateLimiter, TokenUsage,
    VideoAnalysisRequest, VideoAnalysisResponse, CitationRef,
};
use crate::error::{RecallError, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_FILES_API_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";

/// Default request timeout (2 minutes)
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Timeout for file uploads (5 minutes for large files)
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(300);

// Pre-compiled regex patterns for performance
static CITATION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[(\d+)\]").unwrap());
static PAGE_MARKER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[PAGE\s*(\d+)\]").unwrap());

#[derive(Clone)]
pub struct LlmClient {
    client: Client,
    api_key: String,
    embedding_client: EmbeddingClient,
    rate_limiter: Arc<RateLimiter>,
}

impl LlmClient {
    pub fn new(api_key: String) -> Self {
        // Create client with default timeout
        let client = Client::builder()
            .timeout(DEFAULT_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            embedding_client: EmbeddingClient::new(api_key.clone(), "text-embedding-004".to_string()),
            api_key,
            rate_limiter: Arc::new(RateLimiter::new(60)), // 60 RPM default
        }
    }

    pub fn with_rate_limit(mut self, requests_per_minute: u64) -> Self {
        self.rate_limiter = Arc::new(RateLimiter::new(requests_per_minute));
        self
    }

    /// Upload a file to Gemini's Files API for use in generation
    /// Uses resumable upload protocol for reliability
    async fn upload_file(&self, data: &[u8], mime_type: &str, display_name: &str) -> Result<String> {
        self.rate_limiter.wait().await;

        // Step 1: Initiate resumable upload
        let init_url = format!(
            "{}?key={}",
            GEMINI_FILES_API_URL, self.api_key
        );

        let metadata = serde_json::json!({
            "file": {
                "displayName": display_name
            }
        });

        let init_response = self
            .client
            .post(&init_url)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", data.len().to_string())
            .header("X-Goog-Upload-Header-Content-Type", mime_type)
            .header("Content-Type", "application/json")
            .timeout(UPLOAD_TIMEOUT)
            .body(metadata.to_string())
            .send()
            .await?;

        if !init_response.status().is_success() {
            let status = init_response.status();
            let error_text = init_response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(RecallError::RateLimit(60));
            }

            return Err(RecallError::LlmApi(format!(
                "File upload init failed {}: {}",
                status, error_text
            )));
        }

        // Get upload URL from response header
        let upload_url = init_response
            .headers()
            .get("x-goog-upload-url")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| RecallError::LlmApi("No upload URL in init response".to_string()))?
            .to_string();

        // Step 2: Upload the file data
        let upload_response = self
            .client
            .post(&upload_url)
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("X-Goog-Upload-Offset", "0")
            .header("Content-Type", mime_type)
            .timeout(UPLOAD_TIMEOUT)
            .body(data.to_vec())
            .send()
            .await?;

        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let error_text = upload_response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(RecallError::RateLimit(60));
            }

            return Err(RecallError::LlmApi(format!(
                "File upload failed {}: {}",
                status, error_text
            )));
        }

        let response_json: serde_json::Value = upload_response.json().await?;

        // Extract file URI from response
        let file_uri = response_json
            .get("file")
            .and_then(|f| f.get("uri"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| RecallError::LlmApi("No file URI in upload response".to_string()))?
            .to_string();

        tracing::info!("Uploaded file to Gemini: {}", file_uri);
        Ok(file_uri)
    }

    /// Delete a file from Gemini's Files API
    async fn delete_file(&self, file_uri: &str) -> Result<()> {
        // Extract file name from URI (format: files/xxxxx)
        let file_name = file_uri
            .split('/')
            .last()
            .ok_or_else(|| RecallError::LlmApi("Invalid file URI".to_string()))?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/files/{}?key={}",
            file_name, self.api_key
        );

        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Failed to delete file {}: {}", file_uri, response.status());
        }

        Ok(())
    }

    async fn generate_content(
        &self,
        model: &str,
        contents: Vec<GeminiContent>,
        system_instruction: Option<&str>,
        generation_config: Option<GenerationConfig>,
    ) -> Result<GeminiResponse> {
        self.rate_limiter.wait().await;

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_URL, model, self.api_key
        );

        let request = GeminiRequest {
            contents,
            system_instruction: system_instruction.map(|s| SystemInstruction {
                parts: vec![GeminiPart::Text { text: s.to_string() }],
            }),
            generation_config,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            if status.as_u16() == 429 {
                return Err(RecallError::RateLimit(60));
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(RecallError::InvalidApiKey);
            }

            return Err(RecallError::LlmApi(format!(
                "API error {}: {}",
                status, response_text
            )));
        }

        // Parse JSON with better error context
        let gemini_response: GeminiResponse = serde_json::from_str(&response_text)
            .map_err(|e| RecallError::LlmApi(format!(
                "Failed to parse API response: {} - Response: {}",
                e,
                if response_text.len() > 500 { &response_text[..500] } else { &response_text }
            )))?;

        // Log raw response when no candidates returned - use error level for visibility
        if gemini_response.candidates.is_empty() {
            tracing::error!(
                "API returned no candidates. Raw response: {}",
                if response_text.len() > 2000 { &response_text[..2000] } else { &response_text }
            );
        }

        Ok(gemini_response)
    }
}

#[async_trait]
impl LlmProvider for LlmClient {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        // Build context XML
        let context_xml = if !request.context.is_empty() {
            let chunks_xml: String = request
                .context
                .iter()
                .map(|c| {
                    format!(
                        r#"<chunk id="{}" source="{}"{}{}>{}</chunk>"#,
                        c.id,
                        c.source,
                        c.page.map(|p| format!(r#" page="{}""#, p)).unwrap_or_default(),
                        c.timestamp.map(|t| format!(r#" timestamp="{}""#, t)).unwrap_or_default(),
                        c.content
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("<context>\n{}</context>\n\n", chunks_xml)
        } else {
            String::new()
        };

        let system_prompt = request.system_prompt.unwrap_or_else(|| {
            r#"You are a helpful AI assistant that answers questions based on the provided context.

INSTRUCTIONS:
1. Only use information from the provided context to answer questions
2. If the context doesn't contain relevant information, say so clearly
3. When citing sources, use the format [chunk_id] to reference specific chunks
4. Be concise but thorough in your answers
5. If asked about topics not in the context, explain what information IS available

FORMAT YOUR CITATIONS:
When you use information from a chunk, cite it like this: [123] where 123 is the chunk id."#.to_string()
        });

        let full_prompt = format!("{}{}", context_xml, request.prompt);

        // Build contents with conversation history
        let mut contents: Vec<GeminiContent> = Vec::new();

        // Add conversation history (previous messages)
        for msg in &request.history {
            let role = if msg.role == "user" { "user" } else { "model" };
            contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart::Text { text: msg.content.clone() }],
            });
        }

        // Add current user prompt with context
        contents.push(GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart::Text { text: full_prompt }],
        });

        let config = GenerationConfig {
            max_output_tokens: request.max_tokens,
            temperature: request.temperature,
            ..Default::default()
        };

        let response = self
            .generate_content("gemini-2.0-flash", contents, Some(&system_prompt), Some(config))
            .await?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
            .map(|p| match p {
                GeminiPart::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        // Parse citations from content (looking for [id] patterns)
        let citations = parse_citations(&content);

        let usage = response.usage_metadata.map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
        }).unwrap_or(TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(GenerateResponse {
            content,
            citations,
            usage,
        })
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.embedding_client.embed_batch(texts).await
    }

    async fn analyze_video(&self, request: VideoAnalysisRequest) -> Result<VideoAnalysisResponse> {
        if request.frames.is_empty() {
            return Ok(VideoAnalysisResponse { segments: vec![] });
        }

        // Build multimodal content with frames
        let mut parts: Vec<GeminiPart> = Vec::new();

        parts.push(GeminiPart::Text {
            text: format!(
                r#"Analyze these video frames extracted at regular intervals. For each distinct scene or topic change, provide:
1. Start and end timestamps
2. A description of what's happening
3. Key topics or keywords

Video path: {}
Total frames: {}
First frame timestamp: {:.1}s
Last frame timestamp: {:.1}s

Respond in JSON format:
{{
  "segments": [
    {{"start_time": 0.0, "end_time": 30.0, "description": "...", "topics": ["topic1", "topic2"]}}
  ]
}}"#,
                request.video_path,
                request.frames.len(),
                request.frames.first().map(|f| f.timestamp).unwrap_or(0.0),
                request.frames.last().map(|f| f.timestamp).unwrap_or(0.0),
            ),
        });

        // Add frames as inline images (limit to avoid context overflow)
        let max_frames = 20;
        let step = if request.frames.len() > max_frames {
            request.frames.len() / max_frames
        } else {
            1
        };

        for (i, frame) in request.frames.iter().enumerate() {
            if i % step != 0 {
                continue;
            }

            parts.push(GeminiPart::InlineData {
                inline_data: InlineData {
                    mime_type: "image/jpeg".to_string(),
                    data: BASE64.encode(&frame.image_data),
                },
            });

            parts.push(GeminiPart::Text {
                text: format!("Frame at {:.1}s:", frame.timestamp),
            });
        }

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        let config = GenerationConfig {
            response_mime_type: Some("application/json".to_string()),
            ..Default::default()
        };

        // Use retry logic for video analysis to handle rate limits
        let response = self
            .generate_content_with_retry("gemini-2.0-flash", contents, None, Some(config), 5)
            .await?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
            .map(|p| match p {
                GeminiPart::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        // Parse JSON response
        let analysis: VideoAnalysisResponse =
            serde_json::from_str(&content).unwrap_or(VideoAnalysisResponse { segments: vec![] });

        Ok(analysis)
    }

    async fn transcribe_audio(&self, audio_data: &[u8]) -> Result<String> {
        let parts = vec![
            GeminiPart::Text {
                text: "Transcribe the following audio. Provide a verbatim transcription with timestamps for each speaker turn or paragraph. Format: [MM:SS] text".to_string(),
            },
            GeminiPart::InlineData {
                inline_data: InlineData {
                    mime_type: "audio/mp3".to_string(),
                    data: BASE64.encode(audio_data),
                },
            },
        ];

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        // Use retry logic for audio transcription to handle rate limits
        let response = self
            .generate_content_with_retry("gemini-2.0-flash", contents, None, None, 5)
            .await?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
            .map(|p| match p {
                GeminiPart::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        Ok(content)
    }
}

/// Truncate a string at a word boundary, ensuring it doesn't exceed max_chars
fn truncate_at_word_boundary(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }

    // Find the last space before max_chars
    let truncate_at = s[..max_chars]
        .rfind(' ')
        .unwrap_or(max_chars);

    s[..truncate_at].trim_end().to_string()
}

fn parse_citations(content: &str) -> Vec<CitationRef> {
    let mut citations = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in CITATION_REGEX.captures_iter(content) {
        if let Ok(id) = cap[1].parse::<i64>() {
            if seen.insert(id) {
                // Find surrounding text as quote
                let start = cap.get(0).unwrap().start();
                let end = cap.get(0).unwrap().end();

                let quote_start = content[..start].rfind('.').map(|i| i + 1).unwrap_or(0);
                let quote_end = content[end..].find('.').map(|i| end + i + 1).unwrap_or(content.len());

                let quote = content[quote_start..quote_end].trim().to_string();

                citations.push(CitationRef { chunk_id: id, quote });
            }
        }
    }

    citations
}

// Gemini API types
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

/// System instruction structure (no role field, unlike GeminiContent)
#[derive(Debug, Clone, Serialize)]
struct SystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: InlineData },
    FileData { file_data: FileData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Default, Clone, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    #[serde(rename = "promptFeedback")]
    prompt_feedback: Option<PromptFeedback>,
}

#[derive(Debug, Deserialize)]
struct PromptFeedback {
    #[serde(rename = "blockReason")]
    block_reason: Option<String>,
    #[serde(rename = "safetyRatings", default)]
    safety_ratings: Vec<SafetyRating>,
}

#[derive(Debug, Deserialize)]
struct SafetyRating {
    category: String,
    probability: String,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount", default)]
    total_token_count: u32,
}

impl LlmClient {
    /// OCR a PDF file using Gemini's Files API for reliable processing
    /// Supports PDFs up to 2GB
    pub async fn ocr_pdf(&self, pdf_data: &[u8]) -> Result<String> {
        // Check file size - Files API supports up to 2GB
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024 * 1024; // 2GB limit
        if pdf_data.len() > MAX_FILE_SIZE {
            return Err(RecallError::Ingestion(format!(
                "PDF too large ({:.1} MB). Maximum supported size is 2 GB.",
                pdf_data.len() as f64 / (1024.0 * 1024.0)
            )));
        }

        tracing::info!("Uploading PDF ({:.1} MB) to Gemini Files API for OCR...",
            pdf_data.len() as f64 / (1024.0 * 1024.0));

        // Upload file to Gemini Files API with retry
        let file_uri = self.upload_file_with_retry(pdf_data, "application/pdf", "document.pdf").await?;

        // Build request using file reference
        let parts = vec![
            GeminiPart::Text {
                text: "Extract ALL text from this PDF document. \
                    This appears to be a scanned or image-based PDF. \
                    Transcribe every word visible on each page, preserving:\n\
                    1. Paragraph structure\n\
                    2. Headers and section titles\n\
                    3. Lists and bullet points\n\
                    4. Table content (as plain text)\n\n\
                    Do NOT summarize. Provide the complete verbatim text content.".to_string(),
            },
            GeminiPart::FileData {
                file_data: FileData {
                    mime_type: "application/pdf".to_string(),
                    file_uri: file_uri.clone(),
                },
            },
        ];

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        let config = GenerationConfig {
            max_output_tokens: Some(8192), // Allow for large documents
            ..Default::default()
        };

        // Generate content with retry
        let result = self.generate_content_with_retry(
            "gemini-2.0-flash",
            contents,
            None,
            Some(config),
            3 // max retries
        ).await;

        // Clean up uploaded file (best effort)
        let _ = self.delete_file(&file_uri).await;

        match result {
            Ok(response) => {
                let content = response
                    .candidates
                    .first()
                    .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
                    .map(|p| match p {
                        GeminiPart::Text { text } => text.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();

                tracing::info!("OCR completed successfully, extracted {} characters", content.len());
                Ok(content)
            }
            Err(e) => Err(e),
        }
    }

    /// Upload a file with retry logic for rate limiting
    async fn upload_file_with_retry(&self, data: &[u8], mime_type: &str, display_name: &str) -> Result<String> {
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match self.upload_file(data, mime_type, display_name).await {
                Ok(uri) => return Ok(uri),
                Err(RecallError::RateLimit(wait_secs)) => {
                    retry_count += 1;
                    if retry_count > max_retries {
                        return Err(RecallError::RateLimit(wait_secs));
                    }

                    let backoff = std::cmp::min(wait_secs * retry_count as u64, 120);
                    tracing::warn!(
                        "Rate limited during file upload, waiting {} seconds (retry {}/{})",
                        backoff, retry_count, max_retries
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Generate content with retry logic for rate limiting
    async fn generate_content_with_retry(
        &self,
        model: &str,
        contents: Vec<GeminiContent>,
        system_instruction: Option<&str>,
        generation_config: Option<GenerationConfig>,
        max_retries: u32,
    ) -> Result<GeminiResponse> {
        let mut retry_count = 0;

        loop {
            match self
                .generate_content(model, contents.clone(), system_instruction, generation_config.clone())
                .await
            {
                Ok(response) => return Ok(response),
                Err(RecallError::RateLimit(wait_secs)) => {
                    retry_count += 1;
                    if retry_count > max_retries {
                        return Err(RecallError::RateLimit(wait_secs));
                    }

                    let backoff = std::cmp::min(wait_secs * retry_count as u64, 120);
                    tracing::warn!(
                        "Rate limited during generation, waiting {} seconds (retry {}/{})",
                        backoff, retry_count, max_retries
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn analyze_image(&self, image_data: &[u8], mime_type: &str) -> Result<String> {
        tracing::info!(
            "analyze_image called: {} bytes, mime_type={}",
            image_data.len(),
            mime_type
        );

        // NOTE: Move OCR instructions into the text prompt, NOT system_instruction.
        // Gemini API does not properly support system_instruction with InlineData (multimodal).
        // All other working multimodal functions (transcribe_audio, analyze_video, ocr_batch)
        // use inline prompts only - this is the pattern that works.
        // Simple prompt - complex prompts may cause empty responses
        let prompt_text = "Extract ALL text from this image. Output only the raw text, preserving formatting. If no text is visible, output: [NO TEXT DETECTED]";

        let encoded_data = BASE64.encode(image_data);

        // Use low temperature for deterministic output (0.1 instead of 0.0 for stability)
        // Use higher token limit to avoid truncating long text from screenshots
        let config = GenerationConfig {
            max_output_tokens: Some(16384),
            temperature: Some(0.1),
            response_mime_type: None,
        };

        // Gemini API has a known bug where it intermittently returns empty candidates
        // See: https://github.com/googleapis/python-genai/issues/1289
        // Workaround: retry up to 5 times with exponential backoff
        let model = "gemini-2.0-flash";
        let max_attempts = 5;

        for attempt in 1..=max_attempts {
            let parts = vec![
                GeminiPart::Text {
                    text: prompt_text.to_string(),
                },
                GeminiPart::InlineData {
                    inline_data: InlineData {
                        mime_type: mime_type.to_string(),
                        data: encoded_data.clone(),
                    },
                },
            ];

            let contents = vec![GeminiContent {
                role: "user".to_string(),
                parts,
            }];

            let response = self
                .generate_content(
                    model,
                    contents,
                    None, // No system_instruction with multimodal content
                    Some(config.clone()),
                )
                .await?;

            // Check for blocking/safety issues
            if let Some(ref feedback) = response.prompt_feedback {
                if let Some(ref reason) = feedback.block_reason {
                    tracing::warn!("Image OCR blocked by API: {}", reason);
                    return Err(RecallError::LlmApi(format!(
                        "Image blocked by safety filter: {}",
                        reason
                    )));
                }
            }

            // Log candidate info for debugging
            if let Some(candidate) = response.candidates.first() {
                tracing::info!(
                    "OCR attempt {}/{} ({}): finish_reason={:?}, has_content={}",
                    attempt,
                    max_attempts,
                    model,
                    candidate.finish_reason,
                    candidate.content.is_some()
                );
            } else {
                tracing::warn!(
                    "OCR attempt {}/{} ({}): no candidates returned. prompt_feedback={:?}",
                    attempt,
                    max_attempts,
                    model,
                    response.prompt_feedback
                );
            }

            // Detailed extraction with logging
            let content = if let Some(candidate) = response.candidates.first() {
                if let Some(ref content_obj) = candidate.content {
                    if content_obj.parts.is_empty() {
                        tracing::warn!("OCR: candidate has content but parts array is empty");
                        String::new()
                    } else if let Some(part) = content_obj.parts.first() {
                        match part {
                            GeminiPart::Text { text } => text.clone(),
                            _ => {
                                tracing::warn!("OCR: first part is not text");
                                String::new()
                            }
                        }
                    } else {
                        String::new()
                    }
                } else {
                    tracing::warn!("OCR: candidate exists but content is None");
                    String::new()
                }
            } else {
                String::new()
            };

            if !content.is_empty() {
                if attempt > 1 {
                    tracing::info!("OCR succeeded on attempt {}", attempt);
                }
                return Ok(content);
            }

            // Check if the response was blocked due to safety
            if let Some(candidate) = response.candidates.first() {
                if let Some(ref reason) = candidate.finish_reason {
                    if reason == "SAFETY" || reason == "RECITATION" {
                        tracing::warn!("OCR blocked with finish_reason: {}", reason);
                        return Err(RecallError::LlmApi(format!(
                            "Image processing blocked: {}",
                            reason
                        )));
                    }
                }
            }

            if attempt < max_attempts {
                // Exponential backoff: 1s, 2s, 4s, 8s
                let delay_ms = 1000 * (1 << (attempt - 1));
                tracing::info!(
                    "OCR attempt {} returned empty, retrying in {}ms...",
                    attempt,
                    delay_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        tracing::error!(
            "Image OCR failed after {} attempts (mime: {})",
            max_attempts,
            mime_type
        );
        Err(RecallError::LlmApi(
            "OCR failed to extract text from image after multiple attempts".to_string(),
        ))
    }

    /// OCR multiple pages in a single request (batched for efficiency)
    /// Sends up to 4 pages per request to minimize API calls
    async fn ocr_batch(&self, pages: &[(u32, &[u8])]) -> Result<Vec<(u32, String)>> {
        if pages.is_empty() {
            return Ok(vec![]);
        }

        let mut parts = vec![GeminiPart::Text {
            text: format!(
                "Extract ALL text from these {} document page(s). \
                For EACH page, output the text in this exact format:\n\n\
                [PAGE X]\n<text from page X>\n\n\
                Rules:\n\
                - Transcribe every word exactly as it appears\n\
                - Preserve paragraph structure and line breaks\n\
                - Include headers, lists, tables (as plain text)\n\
                - Do NOT summarize - provide complete verbatim text\n\
                - If text is unclear, make your best interpretation\n\
                - Output ONLY the extracted text with [PAGE X] markers, no commentary",
                pages.len()
            ),
        }];

        // Add all page images
        for (page_num, image_data) in pages {
            parts.push(GeminiPart::Text {
                text: format!("Page {}:", page_num),
            });
            parts.push(GeminiPart::InlineData {
                inline_data: InlineData {
                    mime_type: "image/jpeg".to_string(),
                    data: BASE64.encode(image_data),
                },
            });
        }

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        let config = GenerationConfig {
            max_output_tokens: Some(8192), // More tokens for multiple pages
            temperature: Some(0.1),
            ..Default::default()
        };

        let response = self
            .generate_content_with_retry("gemini-2.0-flash", contents, None, Some(config), 3)
            .await?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
            .map(|p| match p {
                GeminiPart::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        // Parse the response to extract text for each page
        let mut results = Vec::new();

        let mut current_page: Option<u32> = None;
        let mut current_text = String::new();

        for line in content.lines() {
            if let Some(caps) = PAGE_MARKER_REGEX.captures(line) {
                // Save previous page if exists
                if let Some(page_num) = current_page {
                    if !current_text.trim().is_empty() {
                        results.push((page_num, current_text.trim().to_string()));
                    }
                }
                // Start new page
                current_page = caps[1].parse().ok();
                current_text = String::new();
            } else if current_page.is_some() {
                current_text.push_str(line);
                current_text.push('\n');
            }
        }

        // Don't forget the last page
        if let Some(page_num) = current_page {
            if !current_text.trim().is_empty() {
                results.push((page_num, current_text.trim().to_string()));
            }
        }

        Ok(results)
    }

    /// OCR multiple page images with batching to reduce API calls
    /// Processes 3 pages per request for optimal balance of speed and reliability
    pub async fn ocr_pages_batched(&self, pages: Vec<(u32, Vec<u8>)>) -> Result<String> {
        const BATCH_SIZE: usize = 3; // 3 pages per request - good balance

        let total_pages = pages.len();
        let mut all_results: Vec<(u32, String)> = Vec::new();

        // Process in batches
        for (batch_idx, chunk) in pages.chunks(BATCH_SIZE).enumerate() {
            let start_page = batch_idx * BATCH_SIZE + 1;
            let end_page = (start_page + chunk.len()).min(total_pages);

            tracing::info!(
                "Gemini Vision OCR: Processing pages {}-{} of {} (batch {}/{})",
                start_page,
                end_page,
                total_pages,
                batch_idx + 1,
                (total_pages + BATCH_SIZE - 1) / BATCH_SIZE
            );

            // Create references for the batch
            let batch_refs: Vec<(u32, &[u8])> = chunk
                .iter()
                .map(|(num, data)| (*num, data.as_slice()))
                .collect();

            match self.ocr_batch(&batch_refs).await {
                Ok(batch_results) => {
                    all_results.extend(batch_results);
                }
                Err(e) => {
                    tracing::warn!("Batch OCR failed, falling back to single-page mode: {}", e);
                    // Fall back to processing pages individually
                    for (page_num, image_data) in chunk {
                        if let Ok(text) = self.ocr_single_page(image_data, *page_num).await {
                            if !text.trim().is_empty() {
                                all_results.push((*page_num, text));
                            }
                        }
                    }
                }
            }
        }

        // Sort by page number and combine with markers
        all_results.sort_by_key(|(num, _)| *num);

        let mut all_text = String::new();
        for (page_num, text) in all_results {
            if !all_text.is_empty() {
                all_text.push_str("\n\n--- Page ");
                all_text.push_str(&page_num.to_string());
                all_text.push_str(" ---\n\n");
            }
            all_text.push_str(&text);
        }

        tracing::info!("Gemini Vision OCR completed: {} characters extracted", all_text.len());
        Ok(all_text)
    }

    /// Generate a concise, content-aware title from extracted text
    /// Returns a short title (3-6 words, max ~40 characters) summarizing the content
    ///
    /// Includes retry logic to handle cases where the API returns empty responses.
    /// Returns an error if title generation fails after retries.
    pub async fn generate_title(&self, text: &str, max_chars: usize) -> Result<String> {
        // If text is too short, don't bother with LLM
        let trimmed = text.trim();
        if trimmed.len() < 20 {
            return Ok(trimmed.chars().take(max_chars).collect());
        }

        // Take first ~2000 chars for context (enough to understand content)
        let sample: String = trimmed.chars().take(2000).collect();

        let prompt = format!(
            r#"Write a very short title (3-6 words MAX) for this content. Be extremely brief.
Rules:
- Maximum 6 words, preferably 3-4
- No colons, no subtitles
- No quotes around the title
- Just output the title, nothing else

Content:
{}"#,
            sample
        );

        let parts = vec![GeminiPart::Text { text: prompt }];

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        let config = GenerationConfig {
            max_output_tokens: Some(20),
            temperature: Some(0.2), // More deterministic
            ..Default::default()
        };

        // Retry up to 2 times on empty response
        let mut attempts = 0;
        let max_attempts = 2;

        loop {
            attempts += 1;

            let response = self
                .generate_content("gemini-2.0-flash", contents.clone(), None, Some(config.clone()))
                .await?;

            let title = response
                .candidates
                .first()
                .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
                .map(|p| match p {
                    GeminiPart::Text { text } => text.trim().to_string(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            if !title.is_empty() {
                // Clean up quotes and newlines
                let clean_title = title
                    .trim_matches(|c| c == '"' || c == '\'' || c == '\n' || c == '*')
                    .to_string();

                // Truncate at word boundary if too long
                let truncated = if clean_title.len() > max_chars {
                    truncate_at_word_boundary(&clean_title, max_chars)
                } else {
                    clean_title
                };

                return Ok(truncated);
            }

            if attempts >= max_attempts {
                tracing::warn!(
                    "Title generation returned empty after {} attempts (text sample: {}...)",
                    attempts,
                    &sample.chars().take(100).collect::<String>()
                );
                return Err(RecallError::LlmApi(
                    "Title generation returned empty response".to_string()
                ));
            }

            tracing::debug!("Title generation attempt {} returned empty, retrying...", attempts);
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    /// OCR a single page (fallback for when batching fails)
    async fn ocr_single_page(&self, image_data: &[u8], page_number: u32) -> Result<String> {
        let parts = vec![
            GeminiPart::Text {
                text: "Extract ALL text from this document page. \
                    Transcribe every word exactly as it appears, preserving structure. \
                    Output ONLY the extracted text, no commentary.".to_string(),
            },
            GeminiPart::InlineData {
                inline_data: InlineData {
                    mime_type: "image/jpeg".to_string(),
                    data: BASE64.encode(image_data),
                },
            },
        ];

        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }];

        let config = GenerationConfig {
            max_output_tokens: Some(4096),
            temperature: Some(0.1),
            ..Default::default()
        };

        tracing::info!("Gemini Vision OCR: Processing page {} (single)", page_number);

        let response = self
            .generate_content_with_retry("gemini-2.0-flash", contents, None, Some(config), 3)
            .await?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
            .map(|p| match p {
                GeminiPart::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        Ok(content)
    }
}

pub async fn validate_api_key(api_key: &str) -> Result<bool> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new());

    let url = format!(
        "{}/gemini-2.0-flash:generateContent?key={}",
        GEMINI_API_URL, api_key
    );

    let request = GeminiRequest {
        contents: vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart::Text {
                text: "Say hello".to_string(),
            }],
        }],
        system_instruction: None,
        generation_config: Some(GenerationConfig {
            max_output_tokens: Some(10),
            ..Default::default()
        }),
    };

    let response = client
        .post(&url)
        .json(&request)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    // Check for HTTP-level errors
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(RecallError::InvalidApiKey);
        }
        if status.as_u16() == 429 {
            // Check if it's a quota/billing issue
            if body.contains("RESOURCE_EXHAUSTED") || body.contains("quota") {
                return Err(RecallError::LlmApi(
                    "API quota exceeded. Please enable billing in Google AI Studio or wait for quota reset.".to_string()
                ));
            }
            return Err(RecallError::RateLimit(60));
        }
        tracing::error!("API validation failed: {} - {}", status, body);
        return Err(RecallError::LlmApi(format!("API error {}: {}", status, extract_error_message(&body))));
    }

    // Check for error field in response body (API can return 200 with error)
    if let Ok(error_response) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(error) = error_response.get("error") {
            let error_msg = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown API error");
            tracing::error!("API returned error in body: {}", error_msg);

            // Check for specific error types
            if error_msg.contains("billing") || error_msg.contains("quota") || error_msg.contains("RESOURCE_EXHAUSTED") {
                return Err(RecallError::LlmApi(
                    "Billing not enabled. Please set up billing in Google AI Studio to use the API.".to_string()
                ));
            }
            if error_msg.contains("not found") || error_msg.contains("does not exist") {
                return Err(RecallError::LlmApi(
                    "Model not available. Please ensure gemini-2.0-flash is enabled for your API key.".to_string()
                ));
            }
            return Err(RecallError::LlmApi(error_msg.to_string()));
        }
    }

    // Parse as GeminiResponse
    let parsed: std::result::Result<GeminiResponse, _> = serde_json::from_str(&body);

    match parsed {
        Ok(resp) => {
            // Check for empty candidates
            if resp.candidates.is_empty() {
                // Check if blocked by safety
                if let Some(ref feedback) = resp.prompt_feedback {
                    if let Some(ref reason) = feedback.block_reason {
                        tracing::error!("API blocked request: {}", reason);
                        return Err(RecallError::LlmApi(format!("Request blocked: {}", reason)));
                    }
                }
                tracing::error!("API returned empty candidates: {}", body);
                return Err(RecallError::LlmApi(
                    "API returned no response. This may indicate a billing or quota issue.".to_string()
                ));
            }

            // Verify we actually got text content
            let has_content = resp.candidates.first()
                .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
                .map(|p| match p {
                    GeminiPart::Text { text } => !text.trim().is_empty(),
                    _ => false,
                })
                .unwrap_or(false);

            if !has_content {
                tracing::error!("API returned empty text content: {}", body);
                return Err(RecallError::LlmApi(
                    "API returned empty response. Please verify your API key has proper permissions.".to_string()
                ));
            }

            Ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to parse API response: {} - {}", e, body);
            Err(RecallError::LlmApi(format!("Invalid API response: {}", extract_error_message(&body))))
        }
    }
}

/// Extract a user-friendly error message from API response body
fn extract_error_message(body: &str) -> String {
    // Try to parse as JSON and extract error message
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = json.get("error") {
            if let Some(msg) = error.get("message").and_then(|m| m.as_str()) {
                return msg.to_string();
            }
        }
    }
    // Return truncated body if no structured error
    if body.len() > 200 {
        format!("{}...", &body[..200])
    } else {
        body.to_string()
    }
}
