use crate::database::FileType;
use crate::error::RecallError;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Runtime, State};
use walkdir::WalkDir;

#[derive(serde::Serialize)]
pub struct WatcherStatus {
    pub is_running: bool,
    pub watched_folders: Vec<String>,
    pub auto_ingest_enabled: bool,
}

#[tauri::command]
pub async fn get_watcher_status(
    state: State<'_, Arc<AppState>>,
) -> Result<WatcherStatus, RecallError> {
    let settings = state.settings.read();

    Ok(WatcherStatus {
        is_running: state.watcher_manager.is_running(),
        watched_folders: settings.watched_folders.clone(),
        auto_ingest_enabled: settings.auto_ingest_enabled,
    })
}

#[tauri::command]
pub async fn start_watcher<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), RecallError> {
    state.start_watcher(app_handle)?;
    Ok(())
}

#[tauri::command]
pub async fn stop_watcher(
    state: State<'_, Arc<AppState>>,
) -> Result<(), RecallError> {
    state.stop_watcher();
    Ok(())
}

#[tauri::command]
pub async fn add_watched_folder<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
    folder_path: String,
) -> Result<(), RecallError> {
    tracing::info!("add_watched_folder called: {}", folder_path);

    let path = PathBuf::from(&folder_path);

    if !path.exists() {
        return Err(RecallError::NotFound(format!("Folder not found: {}", folder_path)));
    }

    if !path.is_dir() {
        return Err(RecallError::Config(format!("Not a directory: {}", folder_path)));
    }

    // Add to settings
    {
        let mut settings = state.settings.write();
        if !settings.watched_folders.contains(&folder_path) {
            settings.watched_folders.push(folder_path.clone());
            tracing::info!("Added folder to settings: {}", folder_path);
        } else {
            tracing::info!("Folder already in settings: {}", folder_path);
        }
    }
    state.save_settings()?;

    // If watcher is running, add the folder directly
    if state.watcher_manager.is_running() {
        tracing::info!("Watcher running, adding folder directly");
        state.watcher_manager.add_folder(path.clone())?;
    } else {
        // Start the watcher if auto-ingest is enabled
        let auto_ingest = state.settings.read().auto_ingest_enabled;
        tracing::info!("Watcher not running, auto_ingest={}", auto_ingest);
        if auto_ingest {
            state.start_watcher(app_handle.clone())?;
        }
    }

    // Scan existing files in the folder and ingest them
    tracing::info!("Scanning existing files in folder: {}", folder_path);
    let ingestion_engine = state.ingestion_engine.clone();
    let database = state.database.clone();
    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let mut ingested_count = 0;
        let mut skipped_count = 0;

        for entry in WalkDir::new(&path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();

            // Skip directories
            if file_path.is_dir() {
                continue;
            }

            // Check file type
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let file_type = FileType::from_extension(ext);
            if matches!(file_type, FileType::Unknown) {
                continue;
            }

            let path_str = file_path.to_string_lossy().to_string();

            // Skip if already ingested
            if let Ok(Some(_)) = database.get_document_by_path(&path_str) {
                skipped_count += 1;
                continue;
            }

            tracing::info!("Initial scan: ingesting {:?}", file_path);

            match ingestion_engine.ingest_file(file_path, &app_handle_clone).await {
                Ok(doc) => {
                    tracing::info!("Initial scan: ingested {}", doc.title);
                    ingested_count += 1;
                }
                Err(e) => {
                    tracing::error!("Initial scan: failed to ingest {:?}: {}", file_path, e);
                }
            }
        }

        tracing::info!(
            "Initial folder scan complete: {} ingested, {} skipped (already indexed)",
            ingested_count,
            skipped_count
        );
    });

    Ok(())
}

#[tauri::command]
pub async fn remove_watched_folder(
    state: State<'_, Arc<AppState>>,
    folder_path: String,
) -> Result<(), RecallError> {
    tracing::info!("remove_watched_folder command called: {}", folder_path);
    let path = PathBuf::from(&folder_path);

    // Remove from settings
    {
        let mut settings = state.settings.write();
        settings.watched_folders.retain(|f| f != &folder_path);
    }
    state.save_settings()?;

    // Remove from watcher if running
    if state.watcher_manager.is_running() {
        state.watcher_manager.remove_folder(&path)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_auto_ingest<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), RecallError> {
    {
        let mut settings = state.settings.write();
        settings.auto_ingest_enabled = enabled;
    }
    state.save_settings()?;

    if enabled {
        state.start_watcher(app_handle)?;
    } else {
        state.stop_watcher();
    }

    Ok(())
}
