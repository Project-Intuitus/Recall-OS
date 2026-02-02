use crate::database::Database;
use crate::error::Result;
use crate::ingestion::{FileWatcher, IngestionEngine, WatchEvent};
use parking_lot::RwLock;
use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime, async_runtime};
use tokio::sync::mpsc;

pub struct WatcherManager {
    watcher: RwLock<Option<FileWatcher>>,
    event_rx: RwLock<Option<mpsc::Receiver<WatchEvent>>>,
    watched_paths: RwLock<HashSet<PathBuf>>,
    is_running: RwLock<bool>,
}

impl WatcherManager {
    pub fn new() -> Self {
        Self {
            watcher: RwLock::new(None),
            event_rx: RwLock::new(None),
            watched_paths: RwLock::new(HashSet::new()),
            is_running: RwLock::new(false),
        }
    }

    pub fn start(&self) -> Result<()> {
        let mut watcher_guard = self.watcher.write();
        if watcher_guard.is_some() {
            tracing::info!("File watcher already running");
            return Ok(()); // Already started
        }

        tracing::info!("Starting file watcher...");
        let (watcher, rx) = FileWatcher::new()?;
        *watcher_guard = Some(watcher);
        *self.event_rx.write() = Some(rx);
        *self.is_running.write() = true;

        tracing::info!("File watcher started successfully");
        Ok(())
    }

    pub fn stop(&self) {
        tracing::info!("stop() called - stopping file watcher");
        *self.watcher.write() = None;
        *self.event_rx.write() = None;
        *self.is_running.write() = false;
        self.watched_paths.write().clear();
        tracing::info!("File watcher stopped");
    }

    pub fn add_folder(&self, path: PathBuf) -> Result<()> {
        let mut watcher_guard = self.watcher.write();
        if let Some(ref mut watcher) = *watcher_guard {
            watcher.watch(&path)?;
            self.watched_paths.write().insert(path);
        }
        Ok(())
    }

    pub fn remove_folder(&self, path: &PathBuf) -> Result<()> {
        tracing::info!("remove_folder called for: {:?}", path);
        let mut watcher_guard = self.watcher.write();
        if let Some(ref mut watcher) = *watcher_guard {
            watcher.unwatch(path)?;
            self.watched_paths.write().remove(path);
        }
        Ok(())
    }

    pub fn get_watched_folders(&self) -> Vec<PathBuf> {
        self.watched_paths.read().iter().cloned().collect()
    }

    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }

    /// Start the background task that processes file events
    pub fn spawn_event_processor<R: Runtime + 'static>(
        &self,
        app_handle: AppHandle<R>,
        ingestion_engine: Arc<IngestionEngine>,
        database: Arc<Database>,
    ) {
        let mut rx = match self.event_rx.write().take() {
            Some(rx) => {
                tracing::info!("Event processor: Got receiver");
                rx
            }
            None => {
                tracing::warn!("Event processor: No receiver available");
                return;
            }
        };

        async_runtime::spawn(async move {
            tracing::info!("File watcher event processor started - waiting for events");

            // Debounce tracking: path -> last event time
            let mut pending_files: HashMap<PathBuf, Instant> = HashMap::new();
            // Files currently being processed (to avoid duplicate processing)
            let mut processing_files: HashSet<PathBuf> = HashSet::new();

            // Debounce delay - wait this long after last event before processing
            const DEBOUNCE_DELAY: Duration = Duration::from_secs(2);

            loop {
                // Use a timeout to periodically check for debounced files ready to process
                match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
                    Ok(Some(event)) => {
                        match event {
                            WatchEvent::Created(path) | WatchEvent::Modified(path) => {
                                // Skip if already being processed
                                if processing_files.contains(&path) {
                                    tracing::debug!("File already being processed, skipping event: {:?}", path);
                                    continue;
                                }

                                // Update debounce timestamp (reset timer on each event)
                                pending_files.insert(path.clone(), Instant::now());
                                tracing::debug!("Debouncing file event: {:?}", path);
                            }
                            WatchEvent::Deleted(path) => {
                                // Handle deletes immediately (no debounce needed)
                                pending_files.remove(&path);
                                processing_files.remove(&path);

                                let path_str = path.to_string_lossy().to_string();
                                tracing::info!("File deleted, removing from index: {:?}", path);

                                if let Ok(Some(doc)) = database.get_document_by_path(&path_str) {
                                    if let Err(e) = database.delete_document(&doc.id) {
                                        tracing::error!("Failed to delete document: {}", e);
                                    } else {
                                        let _ = app_handle.emit("document-deleted", &doc.id);
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Channel closed, exit loop
                        break;
                    }
                    Err(_) => {
                        // Timeout - check for files ready to process
                    }
                }

                // Process files that have been debounced long enough
                let now = Instant::now();
                let ready_files: Vec<PathBuf> = pending_files
                    .iter()
                    .filter(|(_, last_event)| now.duration_since(**last_event) >= DEBOUNCE_DELAY)
                    .map(|(path, _)| path.clone())
                    .collect();

                for path in ready_files {
                    pending_files.remove(&path);

                    // Check if file still exists
                    if !path.exists() {
                        tracing::debug!("File no longer exists after debounce, skipping: {:?}", path);
                        continue;
                    }

                    // Skip if already ingested
                    let path_str = path.to_string_lossy().to_string();
                    if let Ok(Some(existing)) = database.get_document_by_path(&path_str) {
                        tracing::debug!("Document already exists: {} (status: {:?})", existing.title, existing.status);
                        continue;
                    }

                    // Mark as processing to prevent duplicate events
                    processing_files.insert(path.clone());

                    tracing::info!("Auto-ingesting file (after debounce): {:?}", path);
                    let _ = app_handle.emit("auto-ingest-start", path_str.clone());

                    match ingestion_engine.ingest_file(&path, &app_handle).await {
                        Ok(doc) => {
                            tracing::info!("Auto-ingested successfully: {}", doc.title);
                            let _ = app_handle.emit("auto-ingest-complete", &doc);
                        }
                        Err(e) => {
                            tracing::error!("Auto-ingest failed for {:?}: {}", path, e);
                            let _ = app_handle.emit("auto-ingest-error", format!("{}: {}", path_str, e));
                        }
                    }

                    // Remove from processing set
                    processing_files.remove(&path);
                }
            }

            tracing::info!("File watcher event processor stopped");
        });
    }
}
