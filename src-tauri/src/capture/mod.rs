//! Screen capture module for RECALL.OS
//!
//! Provides functionality for:
//! - On-demand screenshot capture (button/hotkey)
//! - Periodic automatic capture
//! - Active window capture (default) or full screen
//! - Application whitelist/blacklist filtering
//! - Privacy protection with default blacklist

mod capturer;
mod filter;
mod scheduler;

pub use capturer::{CaptureMode, CaptureResult, Capturer};
pub use filter::{AppFilter, AppFilterMode, AppInfo, get_running_apps};
pub use scheduler::CaptureScheduler;

use crate::database::{Database, Document, DocumentStatus, FileType};
use crate::error::{RecallError, Result};
use crate::ingestion::IngestionEngine;
use crate::llm::LlmClient;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use uuid::Uuid;

/// Settings specific to screen capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSettings {
    /// Whether screen capture is enabled
    pub enabled: bool,
    /// Capture interval in seconds (for periodic capture)
    pub interval_secs: u64,
    /// Capture mode (full screen or active window)
    pub mode: CaptureMode,
    /// App filter mode
    pub filter_mode: AppFilterMode,
    /// List of apps for whitelist/blacklist
    pub app_list: Vec<String>,
    /// Days to retain captures before auto-deletion
    pub retention_days: u32,
    /// Global hotkey for manual capture
    pub hotkey: String,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 60,
            mode: CaptureMode::ActiveWindow,
            filter_mode: AppFilterMode::None,
            app_list: Vec::new(),
            retention_days: 7,
            hotkey: "Ctrl+Shift+S".to_string(),
        }
    }
}

/// Status of the capture system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus {
    /// Whether capture is enabled in settings
    pub enabled: bool,
    /// Whether periodic capture is currently running
    pub scheduler_running: bool,
    /// Whether capturing is paused
    pub paused: bool,
    /// Capture mode
    pub mode: String,
    /// Capture interval in seconds
    pub interval_secs: u64,
    /// Number of captures taken in this session
    pub capture_count: u64,
    /// Last capture timestamp (if any)
    pub last_capture: Option<String>,
    /// Registered hotkey
    pub hotkey: String,
}

/// Event emitted when a capture is completed
#[derive(Debug, Clone, Serialize)]
pub struct CaptureCompleteEvent {
    pub document_id: String,
    pub file_path: String,
    pub source_app: Option<String>,
    pub window_title: Option<String>,
    /// Content-aware title generated from OCR text
    pub generated_title: Option<String>,
}

/// Event emitted when a capture fails
#[derive(Debug, Clone, Serialize)]
pub struct CaptureErrorEvent {
    pub error: String,
    pub source_app: Option<String>,
}

/// Main capture manager
pub struct CaptureManager {
    /// The capturer instance
    capturer: Capturer,
    /// App filter
    filter: RwLock<AppFilter>,
    /// Current settings
    settings: RwLock<CaptureSettings>,
    /// Scheduler for periodic captures
    scheduler: RwLock<CaptureScheduler>,
    /// Database for storing documents
    database: Arc<Database>,
    /// LLM client for OCR (reserved for future use)
    #[allow(dead_code)]
    llm_client: Arc<RwLock<Option<LlmClient>>>,
    /// Ingestion engine for processing captures
    ingestion_engine: Arc<IngestionEngine>,
    /// Number of captures taken
    capture_count: RwLock<u64>,
    /// Last capture timestamp
    last_capture: RwLock<Option<chrono::DateTime<Utc>>>,
    /// App data directory (reserved for future use)
    #[allow(dead_code)]
    app_data_dir: PathBuf,
}

impl CaptureManager {
    /// Create a new capture manager
    pub fn new(
        app_data_dir: PathBuf,
        database: Arc<Database>,
        llm_client: Arc<RwLock<Option<LlmClient>>>,
        ingestion_engine: Arc<IngestionEngine>,
    ) -> Result<Self> {
        let captures_dir = app_data_dir.join("captures");
        let capturer = Capturer::new(captures_dir)?;
        let settings = CaptureSettings::default();
        let filter = AppFilter::new(settings.filter_mode, settings.app_list.clone());

        Ok(Self {
            capturer,
            filter: RwLock::new(filter),
            settings: RwLock::new(settings),
            scheduler: RwLock::new(CaptureScheduler::new()),
            database,
            llm_client,
            ingestion_engine,
            capture_count: RwLock::new(0),
            last_capture: RwLock::new(None),
            app_data_dir,
        })
    }

    /// Update capture settings
    pub fn update_settings(&self, settings: CaptureSettings) {
        let mut filter = self.filter.write();
        filter.update(settings.filter_mode, settings.app_list.clone());
        *self.settings.write() = settings;
    }

    /// Get current capture settings
    pub fn get_settings(&self) -> CaptureSettings {
        self.settings.read().clone()
    }

    /// Get current capture status
    pub fn get_status(&self) -> CaptureStatus {
        let settings = self.settings.read();
        let scheduler = self.scheduler.read();

        CaptureStatus {
            enabled: settings.enabled,
            scheduler_running: scheduler.is_running(),
            paused: scheduler.is_paused(),
            mode: settings.mode.to_string(),
            interval_secs: settings.interval_secs,
            capture_count: *self.capture_count.read(),
            last_capture: self.last_capture.read().map(|t| t.to_rfc3339()),
            hotkey: settings.hotkey.clone(),
        }
    }

    /// Capture a screenshot now (manual trigger)
    pub async fn capture_now<R: Runtime>(&self, app_handle: &AppHandle<R>) -> Result<CaptureResult> {
        self.capture_and_ingest(app_handle).await
    }

    /// Internal method to capture and ingest a screenshot
    pub async fn capture_and_ingest<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
    ) -> Result<CaptureResult> {
        let mode = {
            let settings = self.settings.read();
            settings.mode
        };

        // For active window mode, check if we should capture based on filter
        if mode == CaptureMode::ActiveWindow {
            let filter = self.filter.read();

            // Get foreground window info to check filter
            if let Some(foreground_app) = get_foreground_app_info() {
                if !filter.should_capture(&foreground_app.process_name, &foreground_app.window_title) {
                    return Err(RecallError::Capture(format!(
                        "Window '{}' matches filter rules, skipping capture",
                        foreground_app.window_title
                    )));
                }
            }
        }

        // Take the screenshot
        let result = self.capturer.capture(mode)?;

        // Update stats
        *self.capture_count.write() += 1;
        *self.last_capture.write() = Some(result.captured_at);

        // Create document record for the screenshot
        let doc = self.create_screenshot_document(&result)?;
        self.database.insert_document(&doc)?;

        // Emit capture started event
        let _ = app_handle.emit("capture-started", &CaptureCompleteEvent {
            document_id: doc.id.clone(),
            file_path: result.file_path.to_string_lossy().to_string(),
            source_app: result.source_app.clone(),
            window_title: result.window_title.clone(),
            generated_title: None, // Title will be generated after OCR
        });

        // Show processing notification
        {
            use crate::notifications::show_processing_notification;
            if let Err(e) = show_processing_notification(app_handle, result.source_app.as_deref()) {
                tracing::warn!("Failed to show processing notification: {}", e);
            }
        }

        // Process the screenshot through ingestion (OCR + indexing)
        // Use ingest_existing_document to process the document we already created
        // This preserves the screenshot metadata and avoids duplicate document creation
        let doc_for_ingestion = doc.clone();
        let ingestion_engine = self.ingestion_engine.clone();
        let app_handle_clone = app_handle.clone();

        // Clone what we need for title generation
        let database = self.database.clone();
        let llm_client = self.llm_client.clone();

        // Spawn async task for ingestion
        tokio::spawn(async move {
            match ingestion_engine.ingest_existing_document(&doc_for_ingestion, &app_handle_clone).await {
                Ok(updated_doc) => {
                    tracing::info!("Screenshot ingested successfully: {}", updated_doc.id);

                    // Generate content-aware title from extracted text
                    let generated_title = Self::generate_content_title(
                        &database,
                        &llm_client,
                        &updated_doc.id,
                    ).await;

                    // Update document title if we generated one
                    if let Some(ref title) = generated_title {
                        if let Err(e) = database.update_document_title(&updated_doc.id, title) {
                            tracing::warn!("Failed to update document title: {}", e);
                        } else {
                            tracing::info!("Updated screenshot title to: {}", title);
                        }
                    }
                    // Note: Related content notification is shown by ingestion engine
                    // via check_and_emit_related_content() if matches are found

                    // Emit capture complete event with generated title
                    let _ = app_handle_clone.emit("capture-complete", &CaptureCompleteEvent {
                        document_id: updated_doc.id.clone(),
                        file_path: updated_doc.file_path.clone(),
                        source_app: doc_for_ingestion.metadata.get("source_app")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        window_title: doc_for_ingestion.metadata.get("window_title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        generated_title,
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to ingest screenshot {}: {}", doc_for_ingestion.id, e);
                    let _ = app_handle_clone.emit("capture-error", CaptureErrorEvent {
                        error: e.to_string(),
                        source_app: doc_for_ingestion.metadata.get("source_app")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        });

        Ok(result)
    }

    /// Create a document record for a screenshot
    fn create_screenshot_document(&self, result: &CaptureResult) -> Result<Document> {
        let file_path = result.file_path.to_string_lossy().to_string();
        let file_size = result.file_size as i64;
        let file_hash = self.compute_file_hash(&result.file_path)?;

        let title = if let Some(ref app) = result.source_app {
            format!("Screenshot - {}", app)
        } else {
            format!("Screenshot - {}", result.captured_at.format("%Y-%m-%d %H:%M:%S"))
        };

        let metadata = serde_json::json!({
            "capture_type": "screenshot",
            "capture_mode": result.mode.to_string(),
            "source_app": result.source_app,
            "window_title": result.window_title,
            "resolution": format!("{}x{}", result.resolution.0, result.resolution.1),
            "captured_at": result.captured_at.to_rfc3339(),
        });

        Ok(Document {
            id: Uuid::new_v4().to_string(),
            title,
            file_path,
            file_type: FileType::Screenshot,
            file_size,
            file_hash,
            mime_type: Some("image/png".to_string()),
            created_at: result.captured_at,
            updated_at: result.captured_at,
            ingested_at: None,
            status: DocumentStatus::Pending,
            error_message: None,
            metadata,
        })
    }

    /// Compute file hash for a screenshot
    fn compute_file_hash(&self, path: &PathBuf) -> Result<String> {
        use sha2::{Digest, Sha256};

        let data = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Generate a content-aware title from the extracted text
    async fn generate_content_title(
        database: &Arc<Database>,
        llm_client: &Arc<RwLock<Option<LlmClient>>>,
        document_id: &str,
    ) -> Option<String> {
        // Get chunks for this document
        let chunks = match database.get_chunks_for_document(document_id) {
            Ok(chunks) => chunks,
            Err(e) => {
                tracing::warn!("Failed to get chunks for title generation: {}", e);
                return None;
            }
        };

        if chunks.is_empty() {
            tracing::debug!("No chunks found for document {}", document_id);
            return None;
        }

        // Combine chunk content (first few chunks should be enough)
        let combined_text: String = chunks
            .iter()
            .take(3) // First 3 chunks should capture the main content
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if combined_text.trim().len() < 20 {
            tracing::debug!("Extracted text too short for title generation");
            return None;
        }

        // Get LLM client
        let client = {
            let guard = llm_client.read();
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
                tracing::info!("Generated content-aware title: {}", title);
                Some(title)
            }
            Ok(_) => {
                tracing::debug!("Generated title was empty");
                None
            }
            Err(e) => {
                tracing::warn!("Failed to generate title: {}", e);
                None
            }
        }
    }

    /// Start periodic capture scheduler
    pub fn start_scheduler<R: Runtime + 'static>(self: &Arc<Self>, app_handle: AppHandle<R>) {
        let settings = self.settings.read();
        if !settings.enabled {
            tracing::info!("Screen capture is disabled, not starting scheduler");
            return;
        }

        let interval = settings.interval_secs;
        drop(settings);

        let mut scheduler = self.scheduler.write();
        scheduler.start(self.clone(), interval, app_handle);
    }

    /// Stop periodic capture scheduler
    pub fn stop_scheduler(&self) {
        // Get scheduler and set it to stop - the actual stopping is synchronous
        let mut scheduler = self.scheduler.write();
        // Use blocking stop instead of async to avoid Send issues with lock guards
        let is_running = scheduler.is_running();
        if is_running {
            // Signal stop and let the scheduler handle cleanup
            scheduler.signal_stop();
        }
    }

    /// Pause periodic captures
    pub fn pause_scheduler(&self) {
        let scheduler = self.scheduler.read();
        scheduler.pause();
    }

    /// Resume periodic captures
    pub fn resume_scheduler(&self) {
        let scheduler = self.scheduler.read();
        scheduler.resume();
    }

    /// Clean up old captures based on retention settings
    pub fn cleanup_old_captures(&self) -> Result<u64> {
        let retention_days = self.settings.read().retention_days;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let captures_dir = self.capturer.captures_dir();

        let mut deleted_count = 0u64;

        if let Ok(entries) = std::fs::read_dir(captures_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(created) = metadata.created() {
                        let created_dt: chrono::DateTime<Utc> = created.into();
                        if created_dt < cutoff {
                            if let Err(e) = std::fs::remove_file(&path) {
                                tracing::warn!("Failed to delete old capture {:?}: {}", path, e);
                            } else {
                                deleted_count += 1;
                                tracing::debug!("Deleted old capture: {:?}", path);
                            }
                        }
                    }
                }
            }
        }

        if deleted_count > 0 {
            tracing::info!("Cleaned up {} old captures", deleted_count);
        }

        Ok(deleted_count)
    }
}

/// Get info about the current foreground application
#[cfg(windows)]
fn get_foreground_app_info() -> Option<AppInfo> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        // Get window title
        let title_len = GetWindowTextLengthW(hwnd);
        let title = if title_len > 0 {
            let mut title_buf: Vec<u16> = vec![0; (title_len + 1) as usize];
            GetWindowTextW(hwnd, &mut title_buf);
            String::from_utf16_lossy(&title_buf[..title_len as usize])
        } else {
            String::new()
        };

        // Get process name
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        let process_name = get_process_name_by_id(process_id).unwrap_or_default();

        Some(AppInfo {
            process_name,
            window_title: title,
            is_foreground: true,
        })
    }
}

#[cfg(windows)]
fn get_process_name_by_id(process_id: u32) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, process_id).ok()?;

        let mut name_buf: [u16; 260] = [0; 260];
        let len = GetModuleBaseNameW(handle, None, &mut name_buf);

        let _ = CloseHandle(handle);

        if len > 0 {
            let os_string = OsString::from_wide(&name_buf[..len as usize]);
            os_string.into_string().ok()
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_foreground_app_info() -> Option<AppInfo> {
    None
}
