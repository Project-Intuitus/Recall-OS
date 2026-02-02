use crate::database::Chunk;
use crate::error::Result;
use chrono::Utc;
use once_cell::sync::Lazy;
use tiktoken_rs::{cl100k_base, CoreBPE};

// Initialize tokenizer once at startup (it's slow to load)
static TOKENIZER: Lazy<CoreBPE> = Lazy::new(|| {
    tracing::info!("Loading cl100k_base tokenizer (this may take a moment on first run)...");
    let bpe = cl100k_base().expect("Failed to load tokenizer");
    tracing::info!("Tokenizer loaded successfully");
    bpe
});

pub struct Chunker {
    chunk_size: usize,
    overlap: usize,
}

impl Chunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }

    pub fn chunk(&self, document_id: &str, content: &ExtractedContent) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let bpe = &*TOKENIZER; // Use pre-loaded tokenizer

        match content {
            ExtractedContent::Text { text, pages } => {
                if let Some(pages) = pages {
                    // Chunk by page, then by token count
                    for (page_num, page_text) in pages.iter().enumerate() {
                        let page_chunks = self.chunk_text(&bpe, page_text);
                        for (_i, (text, token_count)) in page_chunks.into_iter().enumerate() {
                            chunks.push(Chunk {
                                id: 0, // Will be set by database
                                document_id: document_id.to_string(),
                                chunk_index: chunks.len() as i32,
                                content: text,
                                token_count,
                                start_offset: None,
                                end_offset: None,
                                page_number: Some((page_num + 1) as i32),
                                timestamp_start: None,
                                timestamp_end: None,
                                metadata: serde_json::json!({}),
                                created_at: Utc::now(),
                            });
                        }
                    }
                } else {
                    // Chunk entire text
                    let text_chunks = self.chunk_text(&bpe, text);
                    for (i, (text, token_count)) in text_chunks.into_iter().enumerate() {
                        chunks.push(Chunk {
                            id: 0,
                            document_id: document_id.to_string(),
                            chunk_index: i as i32,
                            content: text,
                            token_count,
                            start_offset: None,
                            end_offset: None,
                            page_number: None,
                            timestamp_start: None,
                            timestamp_end: None,
                            metadata: serde_json::json!({}),
                            created_at: Utc::now(),
                        });
                    }
                }
            }
            ExtractedContent::Timed { segments } => {
                // For timed content (video/audio), use segment boundaries
                for segment in segments {
                    let segment_chunks = self.chunk_text(&bpe, &segment.text);
                    let duration = segment.end_time - segment.start_time;
                    let chunk_count = segment_chunks.len().max(1);
                    let time_per_chunk = duration / chunk_count as f64;

                    for (i, (text, token_count)) in segment_chunks.into_iter().enumerate() {
                        let start = segment.start_time + (i as f64 * time_per_chunk);
                        let end = start + time_per_chunk;

                        chunks.push(Chunk {
                            id: 0,
                            document_id: document_id.to_string(),
                            chunk_index: chunks.len() as i32,
                            content: text,
                            token_count,
                            start_offset: None,
                            end_offset: None,
                            page_number: None,
                            timestamp_start: Some(start),
                            timestamp_end: Some(end),
                            metadata: serde_json::json!({
                                "topics": segment.topics,
                            }),
                            created_at: Utc::now(),
                        });
                    }
                }
            }
        }

        Ok(chunks)
    }

    fn chunk_text(&self, bpe: &CoreBPE, text: &str) -> Vec<(String, i32)> {
        // Use character-based chunking for speed, estimate ~4 chars per token
        let chars_per_token = 4;
        let target_chars = self.chunk_size * chars_per_token;
        let overlap_chars = self.overlap * chars_per_token;

        let text_len = text.len();

        if text_len <= target_chars {
            let token_count = bpe.encode_with_special_tokens(text).len();
            return vec![(text.to_string(), token_count as i32)];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text_len {
            let mut end = (start + target_chars).min(text_len);

            // Ensure end is at a valid UTF-8 char boundary
            while end > start && !text.is_char_boundary(end) {
                end -= 1;
            }

            // Try to end at a sentence or word boundary
            if end < text_len && end > start {
                // Look for sentence boundary within last 20% of chunk
                let search_start = end.saturating_sub(target_chars / 5);
                // Ensure search_start is at a valid UTF-8 char boundary
                let search_start = Self::floor_char_boundary(text, search_start);

                if let Some(pos) = text[search_start..end].rfind(|c| c == '.' || c == '!' || c == '?') {
                    end = search_start + pos + 1;
                    // Ensure we're still at a valid boundary after adjustment
                    while end < text_len && !text.is_char_boundary(end) {
                        end += 1;
                    }
                } else if let Some(pos) = text[search_start..end].rfind(' ') {
                    // Fall back to word boundary
                    end = search_start + pos;
                }
            }

            // Final safety check for boundaries
            let start_safe = Self::floor_char_boundary(text, start);
            let end_safe = Self::floor_char_boundary(text, end);

            if end_safe > start_safe {
                let chunk_text = text[start_safe..end_safe].trim().to_string();
                if !chunk_text.is_empty() {
                    let token_count = bpe.encode_with_special_tokens(&chunk_text).len();
                    chunks.push((chunk_text, token_count as i32));
                }
            }

            // Advance with overlap
            let advance = (end - start).saturating_sub(overlap_chars).max(target_chars / 4);
            start += advance;
            // Ensure start is at a valid UTF-8 char boundary
            while start < text_len && !text.is_char_boundary(start) {
                start += 1;
            }
        }

        chunks
    }

    /// Find the largest valid char boundary <= index
    fn floor_char_boundary(text: &str, index: usize) -> usize {
        if index >= text.len() {
            return text.len();
        }
        let mut i = index;
        while i > 0 && !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

}

#[derive(Debug, Clone)]
pub enum ExtractedContent {
    Text {
        text: String,
        pages: Option<Vec<String>>,
    },
    Timed {
        segments: Vec<TimedSegment>,
    },
}

#[derive(Debug, Clone)]
pub struct TimedSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub text: String,
    pub topics: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_basic() {
        let chunker = Chunker::new(512, 50);
        let content = ExtractedContent::Text {
            text: "This is a test. It has multiple sentences. We want to see how chunking works.".to_string(),
            pages: None,
        };

        let chunks = chunker.chunk("doc-1", &content).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks[0].token_count > 0);
    }
}
