use super::chunker::{ExtractedContent, TimedSegment};
use super::ffmpeg::FFmpeg;
use crate::error::{RecallError, Result};
use crate::llm::{LlmClient, LlmProvider, VideoAnalysisRequest, VideoFrame};
use crate::state::Settings;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

/// Progress callback for long-running extraction operations
pub type ProgressCallback = Box<dyn Fn(&str) + Send + Sync>;

// Pre-compiled regex for timestamp parsing
static TIMESTAMP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(\d+):(\d+)\]").unwrap()
});

/// Maximum file size allowed for ingestion (500 MB)
const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

/// Validate file size before reading into memory
fn validate_file_size(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(RecallError::Ingestion(format!(
            "File too large ({:.1} MB). Maximum size is {:.0} MB.",
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_FILE_SIZE as f64 / (1024.0 * 1024.0)
        )));
    }
    Ok(())
}

/// Extract PDF with optional progress callback for UI updates
pub async fn extract_pdf_with_progress(
    path: &Path,
    llm: Option<&LlmClient>,
    on_progress: Option<&ProgressCallback>,
) -> Result<ExtractedContent> {
    validate_file_size(path)?;
    let bytes = std::fs::read(path)?;

    if let Some(cb) = on_progress {
        cb("Reading PDF file...");
    }

    // First try direct text extraction (fast, works for text-based PDFs)
    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => {
            if !text.trim().is_empty() {
                tracing::info!("PDF text extraction successful: {:?}", path);
                let pages = extract_pdf_pages(&bytes);
                return Ok(ExtractedContent::Text { text, pages });
            }
            tracing::warn!("PDF has no extractable text, trying OCR: {:?}", path);
            if let Some(cb) = on_progress {
                cb("No text found, starting OCR...");
            }
        }
        Err(e) => {
            tracing::warn!("PDF text extraction failed, trying OCR: {:?} - {}", path, e);
            if let Some(cb) = on_progress {
                cb("Text extraction failed, trying OCR...");
            }
        }
    }

    // Use Gemini Vision OCR (high quality, requires API key)
    #[cfg(windows)]
    if let Some(llm_client) = llm {
        tracing::info!("Starting Gemini Vision OCR for PDF: {:?}", path);
        if let Some(cb) = on_progress {
            cb("Running Gemini Vision OCR (this may take a while)...");
        }
        match super::windows_ocr::ocr_pdf_gemini_with_progress(path, llm_client, on_progress).await {
            Ok(ocr_text) => {
                if !ocr_text.trim().is_empty() {
                    tracing::info!("Gemini Vision OCR successful: {} characters extracted", ocr_text.len());
                    return Ok(ExtractedContent::Text {
                        text: ocr_text,
                        pages: None,
                    });
                }
                tracing::warn!("Gemini Vision OCR returned empty text, falling back to Windows OCR");
                if let Some(cb) = on_progress {
                    cb("Gemini OCR returned empty, trying Windows OCR...");
                }
            }
            Err(e) => {
                tracing::warn!("Gemini Vision OCR failed: {}, falling back to Windows OCR", e);
                if let Some(cb) = on_progress {
                    cb("Gemini OCR failed, trying Windows OCR...");
                }
            }
        }
    }

    // Fallback to Windows OCR (fast, local, no API calls)
    #[cfg(windows)]
    {
        tracing::info!("Starting Windows OCR for PDF: {:?}", path);
        if let Some(cb) = on_progress {
            cb("Running Windows OCR...");
        }
        match super::windows_ocr::ocr_pdf_windows_with_progress(path, on_progress).await {
            Ok(ocr_text) => {
                if !ocr_text.trim().is_empty() {
                    tracing::info!("Windows OCR successful: {} characters extracted", ocr_text.len());
                    return Ok(ExtractedContent::Text {
                        text: ocr_text,
                        pages: None,
                    });
                }
                tracing::warn!("Windows OCR returned empty text");
            }
            Err(e) => {
                tracing::error!("Windows OCR failed: {}", e);
            }
        }
    }

    // Return error if all methods fail instead of silently returning empty content
    tracing::error!("All PDF extraction methods failed for: {:?}", path);
    Err(RecallError::Ingestion(format!(
        "Failed to extract text from PDF: {:?}. The PDF may be empty, corrupted, or contain only non-text content.",
        path
    )))
}

/// Backward compatible wrapper without progress
pub async fn extract_pdf(path: &Path, llm: Option<&LlmClient>) -> Result<ExtractedContent> {
    extract_pdf_with_progress(path, llm, None).await
}

fn extract_pdf_pages(_bytes: &[u8]) -> Option<Vec<String>> {
    // pdf-extract doesn't directly support page-by-page extraction
    // For now, we'll return None and use the full text
    // A more sophisticated implementation would use lopdf or pdf-rs
    None
}

pub async fn extract_text(path: &Path) -> Result<ExtractedContent> {
    validate_file_size(path)?;
    let text = std::fs::read_to_string(path)?;
    Ok(ExtractedContent::Text { text, pages: None })
}

pub async fn extract_video(
    path: &Path,
    llm: &LlmClient,
    settings: &Settings,
) -> Result<ExtractedContent> {
    let ffmpeg = FFmpeg::new()?;

    // Get video duration
    let duration = ffmpeg.get_duration(path).await?;

    // Extract keyframes at regular intervals
    let interval = settings.keyframe_interval;
    let frames = ffmpeg.extract_keyframes(path, interval).await?;

    if frames.is_empty() {
        return Err(RecallError::FFmpeg("No frames extracted from video".to_string()));
    }

    // Process video in 5-minute segments
    let segment_duration = settings.video_segment_duration as f64;
    let mut all_segments = Vec::new();

    let mut segment_start = 0.0;
    while segment_start < duration {
        let segment_end = (segment_start + segment_duration).min(duration);

        // Get frames for this segment
        let segment_frames: Vec<VideoFrame> = frames
            .iter()
            .filter(|f| f.timestamp >= segment_start && f.timestamp < segment_end)
            .cloned()
            .collect();

        if !segment_frames.is_empty() {
            let request = VideoAnalysisRequest {
                video_path: path.to_string_lossy().to_string(),
                frames: segment_frames,
            };

            let analysis = llm.analyze_video(request).await?;

            for seg in analysis.segments {
                all_segments.push(TimedSegment {
                    start_time: seg.start_time + segment_start,
                    end_time: seg.end_time + segment_start,
                    text: seg.description,
                    topics: seg.topics,
                });
            }
        }

        segment_start = segment_end;
    }

    // Also extract audio and transcribe
    let audio_path = ffmpeg.extract_audio(path).await?;
    // Ensure temp file is cleaned up even on error using a scope guard
    let _audio_cleanup = scopeguard::guard(audio_path.clone(), |path| {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!("Failed to clean up temp audio file {:?}: {}", path, e);
        }
    });

    if let Ok(transcript) = transcribe_audio_file(&audio_path, llm).await {
        // Merge transcript with video analysis
        // For simplicity, add transcript as a single timed segment
        if !transcript.is_empty() {
            all_segments.push(TimedSegment {
                start_time: 0.0,
                end_time: duration,
                text: transcript,
                topics: vec!["transcript".to_string()],
            });
        }
    }

    if all_segments.is_empty() {
        return Err(RecallError::Ingestion("No content extracted from video".to_string()));
    }

    Ok(ExtractedContent::Timed { segments: all_segments })
}

pub async fn extract_audio(path: &Path, llm: &LlmClient) -> Result<ExtractedContent> {
    let ffmpeg = FFmpeg::new()?;

    // Convert to mono MP3 for optimal transcription
    let mono_path = ffmpeg.convert_to_mono_mp3(path).await?;

    // Ensure temp file is cleaned up even on error
    let _mono_cleanup = scopeguard::guard(mono_path.clone(), |path| {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!("Failed to clean up temp mono file {:?}: {}", path, e);
        }
    });

    let transcript = transcribe_audio_file(&mono_path, llm).await?;

    // Parse timestamps from transcript if present
    let segments = parse_transcript_timestamps(&transcript);

    if segments.is_empty() {
        Ok(ExtractedContent::Text {
            text: transcript,
            pages: None,
        })
    } else {
        Ok(ExtractedContent::Timed { segments })
    }
}

async fn transcribe_audio_file(path: &Path, llm: &LlmClient) -> Result<String> {
    let audio_data = std::fs::read(path)?;
    llm.transcribe_audio(&audio_data).await
}

fn parse_transcript_timestamps(transcript: &str) -> Vec<TimedSegment> {
    let mut segments = Vec::new();

    let mut current_time = 0.0;
    let mut current_text = String::new();

    for line in transcript.lines() {
        if let Some(caps) = TIMESTAMP_REGEX.captures(line) {
            // Save previous segment
            if !current_text.is_empty() {
                let next_time = caps[1].parse::<f64>().unwrap_or(0.0) * 60.0
                    + caps[2].parse::<f64>().unwrap_or(0.0);

                segments.push(TimedSegment {
                    start_time: current_time,
                    end_time: next_time,
                    text: current_text.trim().to_string(),
                    topics: vec![],
                });

                current_time = next_time;
                current_text = String::new();
            }

            // Extract text after timestamp
            let text = TIMESTAMP_REGEX.replace(line, "").to_string();
            current_text.push_str(&text);
            current_text.push(' ');
        } else {
            current_text.push_str(line);
            current_text.push(' ');
        }
    }

    // Save final segment
    if !current_text.is_empty() {
        segments.push(TimedSegment {
            start_time: current_time,
            end_time: current_time + 60.0, // Estimate 1 minute for final segment
            text: current_text.trim().to_string(),
            topics: vec![],
        });
    }

    segments
}

pub async fn extract_image(path: &Path, llm: &LlmClient) -> Result<ExtractedContent> {
    validate_file_size(path)?;
    // Read image data
    let image_data = std::fs::read(path)?;

    // Determine MIME type from extension
    let mime_type = mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    tracing::info!(
        "extract_image: path={:?}, size={} bytes, mime_type={}",
        path,
        image_data.len(),
        mime_type
    );

    // Use Gemini to describe the image
    let description = llm.analyze_image(&image_data, &mime_type).await?;

    // Log if no text was detected but still allow indexing
    let trimmed = description.trim();
    if trimmed.is_empty() || trimmed == "[NO TEXT DETECTED]" {
        tracing::info!("Image has no detectable text: {:?}", path);
        // Return placeholder text so the document can still be indexed
        return Ok(ExtractedContent::Text {
            text: "[Image with no detectable text]".to_string(),
            pages: None,
        });
    }

    Ok(ExtractedContent::Text {
        text: description,
        pages: None,
    })
}
