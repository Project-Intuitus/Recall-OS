mod chunker;
mod extractor;
mod ffmpeg;
mod watcher;
mod watcher_manager;
#[cfg(windows)]
mod windows_ocr;

pub use chunker::*;
pub use extractor::*;
pub use ffmpeg::*;
pub use watcher::*;
pub use watcher_manager::*;

use crate::database::{Database, Document, DocumentStatus, FileType, IngestionProgress, IngestionStage};
use crate::llm::LlmProvider;
use crate::error::{RecallError, Result};
use crate::llm::LlmClient;
use crate::rag::{HybridRetriever, RelatedDocument};
use crate::state::Settings;
use chrono::Utc;
use parking_lot::RwLock;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Event emitted when related content is found after ingestion
#[derive(Debug, Clone, Serialize)]
pub struct RelatedContentNotification {
    pub new_document_id: String,
    pub new_document_title: String,
    pub related_documents: Vec<RelatedDocument>,
}

/// Queue entry for pending ingestion
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueuedFile {
    pub path: String,
    pub queued_at: chrono::DateTime<Utc>,
}

pub struct IngestionEngine {
    database: Arc<Database>,
    llm_client: Arc<RwLock<Option<LlmClient>>>,
    settings: Arc<RwLock<Settings>>,
    progress: Arc<RwLock<HashMap<String, IngestionProgress>>>,
    /// Set of document IDs that have been marked for cancellation
    cancelled_docs: Arc<RwLock<std::collections::HashSet<String>>>,
    /// Semaphore to ensure only one ingestion runs at a time
    ingestion_semaphore: Arc<Semaphore>,
    /// Queue of files waiting to be ingested
    pending_queue: Arc<RwLock<Vec<QueuedFile>>>,
}

impl IngestionEngine {
    pub fn new(
        database: Arc<Database>,
        llm_client: Arc<RwLock<Option<LlmClient>>>,
        settings: Arc<RwLock<Settings>>,
    ) -> Self {
        Self {
            database,
            llm_client,
            settings,
            progress: Arc::new(RwLock::new(HashMap::new())),
            cancelled_docs: Arc::new(RwLock::new(std::collections::HashSet::new())),
            // Only allow 1 concurrent ingestion to prevent API rate limiting
            ingestion_semaphore: Arc::new(Semaphore::new(1)),
            pending_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the current queue status
    pub fn get_queue_status(&self) -> (usize, bool) {
        let queue_len = self.pending_queue.read().len();
        let is_processing = self.ingestion_semaphore.available_permits() == 0;
        (queue_len, is_processing)
    }

    /// Get all queued files
    pub fn get_queued_files(&self) -> Vec<QueuedFile> {
        self.pending_queue.read().clone()
    }

    pub async fn ingest_file<R: tauri::Runtime>(
        &self,
        path: &Path,
        app_handle: &tauri::AppHandle<R>,
    ) -> Result<Document> {
        let path_str = path.to_string_lossy().to_string();
        let current_hash = compute_file_hash(path)?;

        // Check if file already exists at this path
        if let Some(existing) = self.database.get_document_by_path(&path_str)? {
            // If completed and unchanged, return existing
            if existing.file_hash == current_hash && existing.status == DocumentStatus::Completed {
                tracing::info!("File already ingested and unchanged: {}", path_str);
                return Ok(existing);
            }
            // Delete old version (changed content OR incomplete/failed status)
            tracing::info!("Re-ingesting file: {} (status: {:?}, hash_changed: {})",
                path_str, existing.status, existing.file_hash != current_hash);
            self.database.delete_document(&existing.id)?;
        }

        // Check if same content exists at a different path (file was renamed)
        if let Some(existing) = self.database.get_document_by_hash(&current_hash)? {
            if existing.status == DocumentStatus::Completed {
                // File was renamed - just update the path
                let new_title = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                tracing::info!(
                    "File renamed: {} -> {} (updating path only)",
                    existing.file_path,
                    path_str
                );

                self.database.update_document_path(&existing.id, &path_str, &new_title)?;

                // Fetch and return the updated document
                return self.database.get_document(&existing.id)?
                    .ok_or_else(|| RecallError::NotFound("Document not found after path update".to_string()));
            }
        }

        // Create document record
        let doc = self.create_document(path)?;
        self.database.insert_document(&doc)?;

        // Add to queue and show queued status
        {
            let mut queue = self.pending_queue.write();
            queue.push(QueuedFile {
                path: path_str.clone(),
                queued_at: Utc::now(),
            });
            let queue_position = queue.len();
            tracing::info!("File queued for ingestion (position {}): {}", queue_position, path_str);
        }

        // Update progress to show queued status with position
        let queue_msg = {
            let queue = self.pending_queue.read();
            if queue.len() > 1 {
                format!("Queued (position {} of {})", queue.len(), queue.len())
            } else {
                "Queued for processing".to_string()
            }
        };
        self.update_progress(&doc.id, &path_str, IngestionStage::Queued, 0.0, &queue_msg);
        self.emit_progress(app_handle, &doc.id);

        // Acquire semaphore to ensure only one file processes at a time
        // This will block until the semaphore is available
        let _permit = self.ingestion_semaphore.acquire().await
            .map_err(|_| RecallError::Ingestion("Ingestion queue closed".to_string()))?;

        // Remove from pending queue now that we're processing
        {
            let mut queue = self.pending_queue.write();
            queue.retain(|q| q.path != path_str);
        }

        tracing::info!("Starting ingestion (semaphore acquired): {}", path_str);

        // Process the file (only one at a time due to semaphore)
        match self.process_document(&doc, app_handle).await {
            Ok(_) => {
                self.database.update_document_status(&doc.id, DocumentStatus::Completed, None)?;
                self.update_progress(&doc.id, &path_str, IngestionStage::Completed, 1.0, "Ingestion complete");
                self.emit_progress(app_handle, &doc.id);

                tracing::info!("Ingestion complete, releasing semaphore: {}", path_str);

                // Generate content-aware title from extracted text
                if let Some(title) = self.generate_content_title(&doc).await {
                    if let Err(e) = self.database.update_document_title(&doc.id, &title) {
                        tracing::warn!("Failed to update document title: {}", e);
                    }
                }

                // Check for related content after successful ingestion
                self.check_and_emit_related_content(&doc, app_handle).await;

                // Add cooldown delay before processing next file to avoid rate limits
                // Only delay if there are more files in the queue
                let queue_len = self.pending_queue.read().len();
                if queue_len > 0 {
                    tracing::info!("Cooldown: waiting 2s before next file ({} remaining in queue)", queue_len);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }

                // Fetch updated document
                self.database.get_document(&doc.id)?
                    .ok_or_else(|| RecallError::NotFound("Document not found after ingestion".to_string()))
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.database.update_document_status(&doc.id, DocumentStatus::Failed, Some(&error_msg))?;
                self.update_progress(&doc.id, &path_str, IngestionStage::Failed, 0.0, &error_msg);
                self.emit_progress(app_handle, &doc.id);

                tracing::error!("Ingestion failed, releasing semaphore: {} - {}", path_str, error_msg);
                Err(e)
            }
        }
        // Semaphore permit is automatically released when _permit goes out of scope
    }

    /// Process an existing document (for screenshots or re-ingestion)
    /// This method is for documents that already exist in the database
    pub async fn ingest_existing_document<R: tauri::Runtime>(
        &self,
        doc: &Document,
        app_handle: &tauri::AppHandle<R>,
    ) -> Result<Document> {
        let path_str = doc.file_path.clone();

        // Add to queue and show queued status
        {
            let mut queue = self.pending_queue.write();
            queue.push(QueuedFile {
                path: path_str.clone(),
                queued_at: Utc::now(),
            });
        }

        // Update progress to show queued status
        self.update_progress(&doc.id, &path_str, IngestionStage::Queued, 0.0, "Queued for processing");
        self.emit_progress(app_handle, &doc.id);

        // Acquire semaphore to ensure only one file processes at a time
        let _permit = self.ingestion_semaphore.acquire().await
            .map_err(|_| RecallError::Ingestion("Ingestion queue closed".to_string()))?;

        // Remove from pending queue now that we're processing
        {
            let mut queue = self.pending_queue.write();
            queue.retain(|q| q.path != path_str);
        }

        tracing::info!("Starting ingestion for existing document: {}", doc.id);

        // Process the file
        match self.process_document(doc, app_handle).await {
            Ok(_) => {
                self.database.update_document_status(&doc.id, DocumentStatus::Completed, None)?;
                self.update_progress(&doc.id, &path_str, IngestionStage::Completed, 1.0, "Ingestion complete");
                self.emit_progress(app_handle, &doc.id);

                tracing::info!("Existing document ingestion complete: {}", doc.id);

                // Generate content-aware title from extracted text (skip screenshots)
                if let Some(title) = self.generate_content_title(doc).await {
                    if let Err(e) = self.database.update_document_title(&doc.id, &title) {
                        tracing::warn!("Failed to update document title: {}", e);
                    }
                }

                // Check for related content after successful ingestion
                self.check_and_emit_related_content(doc, app_handle).await;

                // Fetch updated document
                self.database.get_document(&doc.id)?
                    .ok_or_else(|| RecallError::NotFound("Document not found after ingestion".to_string()))
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.database.update_document_status(&doc.id, DocumentStatus::Failed, Some(&error_msg))?;
                self.update_progress(&doc.id, &path_str, IngestionStage::Failed, 0.0, &error_msg);
                self.emit_progress(app_handle, &doc.id);

                tracing::error!("Existing document ingestion failed: {} - {}", doc.id, error_msg);
                Err(e)
            }
        }
    }

    async fn process_document<R: tauri::Runtime>(
        &self,
        doc: &Document,
        app_handle: &tauri::AppHandle<R>,
    ) -> Result<()> {
        let path = Path::new(&doc.file_path);
        let path_str = doc.file_path.clone();

        // Check for cancellation before starting
        if self.is_cancelled(&doc.id) {
            self.clear_cancelled(&doc.id);
            return Err(RecallError::Ingestion("Ingestion cancelled".to_string()));
        }

        // Update status to processing
        self.database.update_document_status(&doc.id, DocumentStatus::Processing, None)?;

        // Extract text based on file type with type-specific progress messages
        let extraction_msg = match doc.file_type {
            FileType::Pdf => "Extracting PDF (may use OCR)...",
            FileType::Image | FileType::Screenshot => "Running OCR on image...",
            FileType::Video => "Processing video frames...",
            FileType::Audio => "Transcribing audio...",
            _ => "Extracting content...",
        };
        self.update_progress(&doc.id, &path_str, IngestionStage::Extracting, 0.1, extraction_msg);
        self.emit_progress(app_handle, &doc.id);

        // Extract text based on file type
        let extracted = match doc.file_type {
            FileType::Pdf => {
                let llm = {
                    let guard = self.llm_client.read();
                    guard.clone()
                };

                // Create progress callback that updates the UI
                let doc_id = doc.id.clone();
                let path_for_cb = path_str.clone();
                let progress_map = self.progress.clone();
                let app_handle_for_cb = app_handle.clone();

                let progress_callback: extractor::ProgressCallback = Box::new(move |msg: &str| {
                    // Update progress with the message
                    {
                        let mut map = progress_map.write();
                        if let Some(progress) = map.get_mut(&doc_id) {
                            progress.message = msg.to_string();
                        }
                    }
                    // Emit to frontend
                    {
                        let map = progress_map.read();
                        if let Some(progress) = map.get(&doc_id) {
                            let _ = app_handle_for_cb.emit("ingestion-progress", progress.clone());
                        }
                    }
                });

                extract_pdf_with_progress(path, llm.as_ref(), Some(&progress_callback)).await?
            }
            FileType::Text | FileType::Markdown => extract_text(path).await?,
            FileType::Video => {
                let (llm, settings) = {
                    let llm_guard = self.llm_client.read();
                    let llm = llm_guard.as_ref().ok_or(RecallError::Config("LLM client not configured".to_string()))?.clone();
                    let settings = self.settings.read().clone();
                    (llm, settings)
                };
                extract_video(path, &llm, &settings).await?
            }
            FileType::Audio => {
                let llm = {
                    let guard = self.llm_client.read();
                    guard.as_ref().ok_or(RecallError::Config("LLM client not configured".to_string()))?.clone()
                };
                extract_audio(path, &llm).await?
            }
            FileType::Image | FileType::Screenshot => {
                let llm = {
                    let guard = self.llm_client.read();
                    guard.as_ref().ok_or(RecallError::Config("LLM client not configured".to_string()))?.clone()
                };
                extract_image(path, &llm).await?
            }
            FileType::Unknown => {
                return Err(RecallError::Ingestion("Unsupported file type".to_string()));
            }
        };

        // Check for cancellation after extraction
        if self.is_cancelled(&doc.id) {
            self.clear_cancelled(&doc.id);
            return Err(RecallError::Ingestion("Ingestion cancelled".to_string()));
        }

        // Chunk the content
        tracing::info!("Starting chunking for document: {}", doc.id);
        self.update_progress(&doc.id, &path_str, IngestionStage::Chunking, 0.3, "Splitting into chunks...");
        self.emit_progress(app_handle, &doc.id);

        let (chunk_size, chunk_overlap) = {
            let settings = self.settings.read();
            (settings.chunk_size, settings.chunk_overlap)
        };
        let chunker = Chunker::new(chunk_size, chunk_overlap);

        let chunks = chunker.chunk(&doc.id, &extracted)?;
        tracing::info!("Chunking complete: {} chunks created", chunks.len());

        if chunks.is_empty() {
            return Err(RecallError::Ingestion("No content extracted from file".to_string()));
        }

        // Insert chunks
        tracing::info!("Inserting {} chunks into database", chunks.len());
        let chunk_ids = self.database.insert_chunks(&chunks)?;
        tracing::info!("Chunks inserted successfully");

        // Check for cancellation after chunking
        if self.is_cancelled(&doc.id) {
            self.clear_cancelled(&doc.id);
            return Err(RecallError::Ingestion("Ingestion cancelled".to_string()));
        }

        // Generate embeddings
        tracing::info!("Starting embedding generation");
        let embedding_msg = format!("Generating embeddings for {} chunks...", chunks.len());
        self.update_progress(&doc.id, &path_str, IngestionStage::Embedding, 0.5, &embedding_msg);
        self.emit_progress(app_handle, &doc.id);

        // Clone LLM client to avoid holding lock across await
        let llm_client = {
            let guard = self.llm_client.read();
            guard.clone()
        };

        if let Some(ref client) = llm_client {
            let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
            tracing::info!("Calling embed API for {} texts", texts.len());

            // Batch embeddings
            let embeddings = client.embed(&texts).await?;
            tracing::info!("Embeddings received: {} vectors", embeddings.len());

            // Store embeddings
            self.update_progress(&doc.id, &path_str, IngestionStage::Indexing, 0.8, "Indexing vectors");
            self.emit_progress(app_handle, &doc.id);

            tracing::info!("Inserting embeddings into database");
            self.database.insert_embeddings(&chunk_ids, &embeddings)?;
            tracing::info!("Embeddings inserted successfully");
        } else {
            tracing::warn!("LLM client not configured, skipping embeddings");
        }

        Ok(())
    }

    fn create_document(&self, path: &Path) -> Result<Document> {
        let metadata = std::fs::metadata(path)?;
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let file_type = FileType::from_extension(extension);
        let file_hash = compute_file_hash(path)?;

        Ok(Document {
            id: Uuid::new_v4().to_string(),
            title: file_name,
            file_path: path.to_string_lossy().to_string(),
            file_type,
            file_size: metadata.len() as i64,
            file_hash,
            mime_type: mime_guess::from_path(path)
                .first()
                .map(|m| m.to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ingested_at: None,
            status: DocumentStatus::Pending,
            error_message: None,
            metadata: serde_json::json!({}),
        })
    }

    fn update_progress(&self, doc_id: &str, file_path: &str, stage: IngestionStage, progress: f64, message: &str) {
        let mut progress_map = self.progress.write();
        progress_map.insert(doc_id.to_string(), IngestionProgress {
            document_id: doc_id.to_string(),
            file_path: file_path.to_string(),
            stage,
            progress,
            message: message.to_string(),
        });
    }

    fn emit_progress<R: tauri::Runtime>(&self, app_handle: &tauri::AppHandle<R>, doc_id: &str) {
        let progress_map = self.progress.read();
        if let Some(progress) = progress_map.get(doc_id) {
            let _ = app_handle.emit("ingestion-progress", progress);
        }
    }

    /// Check for related content and emit notification if found
    async fn check_and_emit_related_content<R: tauri::Runtime>(
        &self,
        doc: &Document,
        app_handle: &tauri::AppHandle<R>,
    ) {
        // Get LLM client for similarity search
        let llm = {
            let guard = self.llm_client.read();
            match guard.as_ref() {
                Some(client) => client.clone(),
                None => return, // No LLM client, skip related content check
            }
        };

        // Check if there are other documents to compare against
        let doc_count = match self.database.get_all_documents() {
            Ok(docs) => docs.len(),
            Err(_) => return,
        };

        // Only check for related content if there are other documents
        if doc_count <= 1 {
            return;
        }

        // Find related documents
        let retriever = HybridRetriever::new(self.database.clone(), llm);
        match retriever.find_related_documents(&doc.id, 5, 0.3).await {
            Ok(related) if !related.is_empty() => {
                tracing::info!(
                    "Found {} related documents for '{}'",
                    related.len(),
                    doc.title
                );

                let notification = RelatedContentNotification {
                    new_document_id: doc.id.clone(),
                    new_document_title: doc.title.clone(),
                    related_documents: related,
                };

                if let Err(e) = app_handle.emit("related-content-found", &notification) {
                    tracing::warn!("Failed to emit related content notification: {}", e);
                }

                // Show custom notification window with rich styling
                {
                    use crate::notifications::show_related_content_notification;

                    let related_info: Vec<(String, String, f64)> = notification.related_documents
                        .iter()
                        .map(|d| (d.id.clone(), d.title.clone(), d.similarity))
                        .collect();

                    if let Err(e) = show_related_content_notification(
                        app_handle,
                        &notification.new_document_id,
                        &notification.new_document_title,
                        &related_info,
                    ) {
                        tracing::warn!("Failed to show notification window: {}", e);
                    }
                }
            }
            Ok(_) => {
                tracing::debug!("No related content found for '{}'", doc.title);
            }
            Err(e) => {
                tracing::warn!("Failed to check for related content: {}", e);
            }
        }
    }

    /// Generate a content-aware title from the extracted text
    /// This now handles all file types including screenshots (for reingest support)
    async fn generate_content_title(&self, doc: &Document) -> Option<String> {
        // Get chunks for this document
        let chunks = match self.database.get_chunks_for_document(&doc.id) {
            Ok(chunks) => chunks,
            Err(e) => {
                tracing::warn!("Failed to get chunks for title generation: {}", e);
                return None;
            }
        };

        if chunks.is_empty() {
            return None;
        }

        // Combine chunk content (first few chunks should capture the main content)
        let combined_text: String = chunks
            .iter()
            .take(3)
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if combined_text.trim().len() < 50 {
            tracing::debug!("Extracted text too short for title generation");
            return None;
        }

        // Get LLM client
        let client = {
            let guard = self.llm_client.read();
            guard.clone()
        };

        let client = match client {
            Some(c) => c,
            None => {
                tracing::debug!("No LLM client available for title generation");
                return None;
            }
        };

        // Generate title (max 40 characters)
        match client.generate_title(&combined_text, 40).await {
            Ok(title) if !title.is_empty() => {
                tracing::info!("Generated content-aware title for {}: {}", doc.id, title);
                Some(title)
            }
            Ok(_) => {
                tracing::debug!("Title generation returned empty string for {}", doc.id);
                None
            }
            Err(e) => {
                tracing::warn!("Title generation failed for {}: {}", doc.id, e);
                None
            }
        }
    }

    pub fn get_progress(&self, doc_id: &str) -> Option<IngestionProgress> {
        self.progress.read().get(doc_id).cloned()
    }

    pub fn get_all_progress(&self) -> Vec<IngestionProgress> {
        self.progress.read().values().cloned().collect()
    }

    /// Clear all progress and cancelled state (used during database reset)
    pub fn clear_all_progress(&self) {
        tracing::info!("Clearing all ingestion progress");
        self.progress.write().clear();
        self.cancelled_docs.write().clear();
        self.pending_queue.write().clear();
    }

    /// Request cancellation of a document's ingestion
    pub fn cancel(&self, doc_id: &str) -> bool {
        tracing::info!("Cancellation requested for document: {}", doc_id);
        self.cancelled_docs.write().insert(doc_id.to_string());
        true
    }

    /// Check if a document's ingestion has been cancelled
    pub fn is_cancelled(&self, doc_id: &str) -> bool {
        self.cancelled_docs.read().contains(doc_id)
    }

    /// Clear cancellation status for a document (after cleanup)
    pub fn clear_cancelled(&self, doc_id: &str) {
        self.cancelled_docs.write().remove(doc_id);
    }
}

/// Maximum file size allowed for ingestion (500 MB)
const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

fn compute_file_hash(path: &Path) -> Result<String> {
    // Check file size before reading to prevent OOM
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(RecallError::Ingestion(format!(
            "File too large ({:.1} MB). Maximum size is {:.0} MB.",
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_FILE_SIZE as f64 / (1024.0 * 1024.0)
        )));
    }

    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}
