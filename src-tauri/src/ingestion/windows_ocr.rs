//! Windows OCR implementation using built-in Windows APIs
//! - Windows.Data.Pdf for PDF rendering (no external DLLs needed)
//! - Windows.Media.Ocr for text extraction
//! Fast, offline, no API rate limits

use crate::error::{RecallError, Result};
use std::path::Path;

#[cfg(windows)]
use windows::{
    core::HSTRING,
    Data::Pdf::{PdfDocument, PdfPageRenderOptions},
    Graphics::Imaging::{BitmapDecoder, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::{StorageFile, Streams::InMemoryRandomAccessStream},
};

/// Scale factor for rendering PDFs for Windows OCR (higher = better quality but slower)
/// 3.0 = 216 DPI equivalent - needed for character-level OCR accuracy
const RENDER_SCALE: f64 = 3.0;

/// Scale factor for Gemini Vision OCR (lower is fine - AI understands context)
/// 2.0 = 144 DPI equivalent - sufficient for Gemini's vision capabilities
const GEMINI_RENDER_SCALE: f64 = 2.0;

/// Extract text from a PDF using Windows built-in APIs with progress callback
#[cfg(windows)]
pub async fn ocr_pdf_windows_with_progress(
    pdf_path: &Path,
    on_progress: Option<&super::extractor::ProgressCallback>,
) -> Result<String> {
    tracing::info!("Starting Windows OCR for PDF: {:?}", pdf_path);

    if let Some(cb) = on_progress {
        cb("Starting Windows OCR...");
    }

    let path_owned = pdf_path.to_path_buf();

    // Run the entire OCR process in a blocking thread
    // Windows COM APIs don't play well with tokio's async runtime
    let result = tokio::task::spawn_blocking(move || {
        ocr_pdf_sync(&path_owned)
    })
    .await
    .map_err(|e| RecallError::Ocr(format!("Task join error: {}", e)))?;

    result
}

/// Extract text from a PDF using Windows built-in APIs (backward compatible)
#[cfg(windows)]
pub async fn ocr_pdf_windows(pdf_path: &Path) -> Result<String> {
    ocr_pdf_windows_with_progress(pdf_path, None).await
}

/// Synchronous OCR implementation
#[cfg(windows)]
fn ocr_pdf_sync(pdf_path: &Path) -> Result<String> {
    let path_str = pdf_path.to_string_lossy().to_string();
    let hstring_path = HSTRING::from(&path_str);

    tracing::info!("Opening PDF file: {}", path_str);

    // Open PDF file
    let file = StorageFile::GetFileFromPathAsync(&hstring_path)
        .map_err(|e| RecallError::Ocr(format!("Failed to open PDF file: {}", e)))?
        .get()
        .map_err(|e| RecallError::Ocr(format!("Failed to get PDF file: {}", e)))?;

    tracing::info!("Loading PDF document...");

    // Load PDF document using Windows.Data.Pdf
    let pdf_doc = PdfDocument::LoadFromFileAsync(&file)
        .map_err(|e| RecallError::Ocr(format!("Failed to load PDF: {}", e)))?
        .get()
        .map_err(|e| RecallError::Ocr(format!("Failed to get PDF document: {}", e)))?;

    let page_count = pdf_doc.PageCount()
        .map_err(|e| RecallError::Ocr(format!("Failed to get page count: {}", e)))?;

    tracing::info!("PDF has {} pages", page_count);

    // Get OCR engine
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| RecallError::Ocr(format!("Failed to create OCR engine: {}", e)))?;

    tracing::info!("OCR engine created, processing pages...");

    let mut all_text = String::new();

    // Process each page
    for i in 0..page_count {
        tracing::info!("Processing page {}/{}", i + 1, page_count);

        // Get page
        let page = pdf_doc.GetPage(i)
            .map_err(|e| RecallError::Ocr(format!("Failed to get page {}: {}", i + 1, e)))?;

        // Get page dimensions and calculate scaled size for higher quality OCR
        let page_size = page.Size()
            .map_err(|e| RecallError::Ocr(format!("Failed to get page size: {}", e)))?;

        let scaled_width = (page_size.Width as f64 * RENDER_SCALE) as u32;
        let scaled_height = (page_size.Height as f64 * RENDER_SCALE) as u32;

        // Create render options with higher resolution
        let render_options = PdfPageRenderOptions::new()
            .map_err(|e| RecallError::Ocr(format!("Failed to create render options: {}", e)))?;
        render_options.SetDestinationWidth(scaled_width)
            .map_err(|e| RecallError::Ocr(format!("Failed to set width: {}", e)))?;
        render_options.SetDestinationHeight(scaled_height)
            .map_err(|e| RecallError::Ocr(format!("Failed to set height: {}", e)))?;

        // Create in-memory stream for rendering
        let stream = InMemoryRandomAccessStream::new()
            .map_err(|e| RecallError::Ocr(format!("Failed to create stream: {}", e)))?;

        tracing::debug!("Rendering page {} at {}x{} ({}x scale)...", i + 1, scaled_width, scaled_height, RENDER_SCALE);

        // Render page to stream at higher resolution
        page.RenderWithOptionsToStreamAsync(&stream, &render_options)
            .map_err(|e| RecallError::Ocr(format!("Failed to start render: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to render page {}: {}", i + 1, e)))?;

        // Reset stream position
        stream.Seek(0)
            .map_err(|e| RecallError::Ocr(format!("Failed to seek stream: {}", e)))?;

        tracing::debug!("Decoding bitmap for page {}...", i + 1);

        // Create bitmap decoder from stream
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| RecallError::Ocr(format!("Failed to create decoder: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to get decoder: {}", e)))?;

        // Get software bitmap for OCR
        let bitmap: SoftwareBitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| RecallError::Ocr(format!("Failed to get bitmap: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to decode bitmap: {}", e)))?;

        tracing::debug!("Running OCR on page {}...", i + 1);

        // Run OCR
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| RecallError::Ocr(format!("OCR failed on page {}: {}", i + 1, e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to get OCR result: {}", e)))?;

        // Extract text
        let page_text = result
            .Text()
            .map_err(|e| RecallError::Ocr(format!("Failed to get text: {}", e)))?
            .to_string();

        tracing::info!("Page {} OCR complete: {} characters", i + 1, page_text.len());

        if !page_text.trim().is_empty() {
            if !all_text.is_empty() {
                all_text.push_str("\n\n--- Page ");
                all_text.push_str(&(i + 1).to_string());
                all_text.push_str(" ---\n\n");
            }
            all_text.push_str(&page_text);
        }
    }

    // Post-process to clean up OCR artifacts
    let cleaned_text = clean_ocr_text(&all_text);

    tracing::info!("Windows OCR completed, extracted {} characters (cleaned from {})",
                   cleaned_text.len(), all_text.len());
    Ok(cleaned_text)
}

/// Clean up common OCR artifacts and garbage text
#[cfg(windows)]
fn clean_ocr_text(text: &str) -> String {
    use regex::Regex;

    // Process line by line to filter garbage
    let lines: Vec<&str> = text.lines().collect();
    let mut cleaned_lines: Vec<String> = Vec::new();

    // Regex patterns for garbage detection
    let lorem_pattern = Regex::new(r"(?i)lorem\s+ipsum|dolor\s+sit\s+amet|consectetur\s+adipiscing").unwrap();
    let garbage_ratio_threshold = 0.4; // If >40% of line is non-alphanumeric, likely garbage

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines (but preserve page markers)
        if trimmed.is_empty() {
            cleaned_lines.push(String::new());
            continue;
        }

        // Always keep page markers
        if trimmed.starts_with("--- Page") && trimmed.ends_with("---") {
            cleaned_lines.push(trimmed.to_string());
            continue;
        }

        // Skip Lorem ipsum garbage
        if lorem_pattern.is_match(trimmed) {
            continue;
        }

        // Skip lines that are mostly garbage characters
        let alpha_count = trimmed.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).count();
        let total_count = trimmed.chars().count();
        if total_count > 5 {
            let garbage_ratio = 1.0 - (alpha_count as f64 / total_count as f64);
            if garbage_ratio > garbage_ratio_threshold {
                continue;
            }
        }

        // Skip very short lines that are likely OCR noise (single chars, random symbols)
        if trimmed.len() < 3 && !trimmed.chars().all(|c| c.is_alphanumeric()) {
            continue;
        }

        // Clean up the line
        let mut cleaned = trimmed.to_string();

        // Remove repeated punctuation artifacts
        cleaned = Regex::new(r"[.]{4,}").unwrap().replace_all(&cleaned, "...").to_string();
        cleaned = Regex::new(r"[-]{3,}").unwrap().replace_all(&cleaned, "—").to_string();
        cleaned = Regex::new(r"[*]{3,}").unwrap().replace_all(&cleaned, "").to_string();

        // Clean up excessive whitespace
        cleaned = Regex::new(r"\s{3,}").unwrap().replace_all(&cleaned, "  ").to_string();

        if !cleaned.trim().is_empty() {
            cleaned_lines.push(cleaned);
        }
    }

    // Join and clean up multiple blank lines
    let result = cleaned_lines.join("\n");
    Regex::new(r"\n{3,}").unwrap().replace_all(&result, "\n\n").to_string()
}

/// Fallback for non-Windows platforms
#[cfg(not(windows))]
pub async fn ocr_pdf_windows(_pdf_path: &Path) -> Result<String> {
    Err(RecallError::Ocr("Windows OCR is only available on Windows".to_string()))
}

/// Extract text from a PDF using Gemini Vision API with progress callback
#[cfg(windows)]
pub async fn ocr_pdf_gemini_with_progress(
    pdf_path: &Path,
    llm: &crate::llm::LlmClient,
    on_progress: Option<&super::extractor::ProgressCallback>,
) -> Result<String> {
    tracing::info!("Starting Gemini Vision OCR for PDF: {:?}", pdf_path);

    let path_owned = pdf_path.to_path_buf();

    if let Some(cb) = on_progress {
        cb("Rendering PDF pages...");
    }

    // Render PDF pages to optimized JPEG images in a blocking thread
    let page_images = tokio::task::spawn_blocking(move || {
        render_pdf_pages_to_jpeg(&path_owned)
    })
    .await
    .map_err(|e| RecallError::Ocr(format!("Task join error: {}", e)))??;

    if page_images.is_empty() {
        return Err(RecallError::Ocr("No pages rendered from PDF".to_string()));
    }

    let total_pages = page_images.len();
    let total_size: usize = page_images.iter().map(|(_, d)| d.len()).sum();
    tracing::info!(
        "Rendered {} pages ({} KB total), sending to Gemini Vision...",
        total_pages,
        total_size / 1024
    );

    if let Some(cb) = on_progress {
        cb(&format!("OCR processing {} pages with Gemini...", total_pages));
    }

    // Send pages to Gemini Vision OCR with batching
    let text = llm.ocr_pages_batched(page_images).await?;

    Ok(text)
}

/// Extract text from a PDF using Gemini Vision API (backward compatible)
#[cfg(windows)]
pub async fn ocr_pdf_gemini(pdf_path: &Path, llm: &crate::llm::LlmClient) -> Result<String> {
    ocr_pdf_gemini_with_progress(pdf_path, llm, None).await
}

/// Render PDF pages to optimized JPEG images for Gemini Vision OCR
/// Uses lower resolution and JPEG compression for smaller file sizes
#[cfg(windows)]
fn render_pdf_pages_to_jpeg(pdf_path: &Path) -> Result<Vec<(u32, Vec<u8>)>> {
    use windows::{
        Graphics::Imaging::{BitmapEncoder, BitmapPixelFormat},
        Storage::Streams::{DataReader, InMemoryRandomAccessStream},
    };

    let path_str = pdf_path.to_string_lossy().to_string();
    let hstring_path = HSTRING::from(&path_str);

    tracing::info!("Opening PDF for Gemini Vision rendering: {}", path_str);

    // Open PDF file
    let file = StorageFile::GetFileFromPathAsync(&hstring_path)
        .map_err(|e| RecallError::Ocr(format!("Failed to open PDF file: {}", e)))?
        .get()
        .map_err(|e| RecallError::Ocr(format!("Failed to get PDF file: {}", e)))?;

    // Load PDF document
    let pdf_doc = PdfDocument::LoadFromFileAsync(&file)
        .map_err(|e| RecallError::Ocr(format!("Failed to load PDF: {}", e)))?
        .get()
        .map_err(|e| RecallError::Ocr(format!("Failed to get PDF document: {}", e)))?;

    let page_count = pdf_doc.PageCount()
        .map_err(|e| RecallError::Ocr(format!("Failed to get page count: {}", e)))?;

    tracing::info!("PDF has {} pages (rendering at {}x scale)", page_count, GEMINI_RENDER_SCALE);

    let mut page_images: Vec<(u32, Vec<u8>)> = Vec::new();

    // Process each page
    for i in 0..page_count {
        // Get page
        let page = pdf_doc.GetPage(i)
            .map_err(|e| RecallError::Ocr(format!("Failed to get page {}: {}", i + 1, e)))?;

        // Get page dimensions and calculate scaled size (optimized for Gemini)
        let page_size = page.Size()
            .map_err(|e| RecallError::Ocr(format!("Failed to get page size: {}", e)))?;

        let scaled_width = (page_size.Width as f64 * GEMINI_RENDER_SCALE) as u32;
        let scaled_height = (page_size.Height as f64 * GEMINI_RENDER_SCALE) as u32;

        // Create render options
        let render_options = PdfPageRenderOptions::new()
            .map_err(|e| RecallError::Ocr(format!("Failed to create render options: {}", e)))?;
        render_options.SetDestinationWidth(scaled_width)
            .map_err(|e| RecallError::Ocr(format!("Failed to set width: {}", e)))?;
        render_options.SetDestinationHeight(scaled_height)
            .map_err(|e| RecallError::Ocr(format!("Failed to set height: {}", e)))?;

        // Create in-memory stream for rendering
        let stream = InMemoryRandomAccessStream::new()
            .map_err(|e| RecallError::Ocr(format!("Failed to create stream: {}", e)))?;

        // Render page to stream
        page.RenderWithOptionsToStreamAsync(&stream, &render_options)
            .map_err(|e| RecallError::Ocr(format!("Failed to start render: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to render page {}: {}", i + 1, e)))?;

        stream.Seek(0)
            .map_err(|e| RecallError::Ocr(format!("Failed to seek stream: {}", e)))?;

        // Decode the rendered image
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| RecallError::Ocr(format!("Failed to create decoder: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to get decoder: {}", e)))?;

        let bitmap = decoder.GetSoftwareBitmapAsync()
            .map_err(|e| RecallError::Ocr(format!("Failed to get bitmap: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to decode bitmap: {}", e)))?;

        // Convert to BGRA8 format for JPEG encoding
        let converted_bitmap = SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)
            .map_err(|e| RecallError::Ocr(format!("Failed to convert bitmap format: {}", e)))?;

        // Create output stream for JPEG
        let jpeg_stream = InMemoryRandomAccessStream::new()
            .map_err(|e| RecallError::Ocr(format!("Failed to create JPEG stream: {}", e)))?;

        // Encode as JPEG with quality setting
        let encoder = BitmapEncoder::CreateAsync(
            BitmapEncoder::JpegEncoderId()
                .map_err(|e| RecallError::Ocr(format!("Failed to get JPEG encoder ID: {}", e)))?,
            &jpeg_stream,
        )
        .map_err(|e| RecallError::Ocr(format!("Failed to create JPEG encoder: {}", e)))?
        .get()
        .map_err(|e| RecallError::Ocr(format!("Failed to get JPEG encoder: {}", e)))?;

        encoder.SetSoftwareBitmap(&converted_bitmap)
            .map_err(|e| RecallError::Ocr(format!("Failed to set bitmap for encoding: {}", e)))?;

        // Use default JPEG quality (good balance of size and quality for OCR)
        encoder.FlushAsync()
            .map_err(|e| RecallError::Ocr(format!("Failed to start flush: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to encode JPEG: {}", e)))?;

        // Read JPEG data from stream
        jpeg_stream.Seek(0)
            .map_err(|e| RecallError::Ocr(format!("Failed to seek JPEG stream: {}", e)))?;

        let size = jpeg_stream.Size()
            .map_err(|e| RecallError::Ocr(format!("Failed to get stream size: {}", e)))? as u32;

        let input_stream = jpeg_stream.GetInputStreamAt(0)
            .map_err(|e| RecallError::Ocr(format!("Failed to get input stream: {}", e)))?;

        let reader = DataReader::CreateDataReader(&input_stream)
            .map_err(|e| RecallError::Ocr(format!("Failed to create data reader: {}", e)))?;

        reader.LoadAsync(size)
            .map_err(|e| RecallError::Ocr(format!("Failed to load data: {}", e)))?
            .get()
            .map_err(|e| RecallError::Ocr(format!("Failed to read data: {}", e)))?;

        let mut jpeg_data = vec![0u8; size as usize];
        reader.ReadBytes(&mut jpeg_data)
            .map_err(|e| RecallError::Ocr(format!("Failed to read JPEG bytes: {}", e)))?;

        tracing::info!("Page {}/{} rendered: {} KB JPEG", i + 1, page_count, jpeg_data.len() / 1024);
        page_images.push((i + 1, jpeg_data));
    }

    Ok(page_images)
}

/// Fallback for non-Windows platforms
#[cfg(not(windows))]
pub async fn ocr_pdf_gemini(_pdf_path: &Path, _llm: &crate::llm::LlmClient) -> Result<String> {
    Err(RecallError::Ocr("Gemini Vision OCR requires Windows for PDF rendering".to_string()))
}
