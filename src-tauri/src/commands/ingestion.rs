use crate::database::{Document, IngestionProgress};
use crate::error::RecallError;
use crate::ingestion::QueuedFile;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};
use walkdir::WalkDir;

#[tauri::command]
pub async fn ingest_file(
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
    path: String,
) -> Result<Document, RecallError> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(RecallError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }

    // Trial limit is enforced inside IngestionEngine::ingest_file()
    state.ingestion_engine.ingest_file(&path, &app_handle).await
}

#[tauri::command]
pub async fn ingest_directory(
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
    path: String,
    recursive: Option<bool>,
) -> Result<Vec<Document>, RecallError> {
    let path = PathBuf::from(path);

    if !path.exists() || !path.is_dir() {
        return Err(RecallError::NotFound(format!(
            "Directory not found: {}",
            path.display()
        )));
    }

    let recursive = recursive.unwrap_or(true);
    let mut documents = Vec::new();
    let mut errors = Vec::new();

    let walker = if recursive {
        WalkDir::new(&path)
    } else {
        WalkDir::new(&path).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let file_path = entry.path();

            // Skip hidden files and unsupported types
            if file_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(true)
            {
                continue;
            }

            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let file_type = crate::database::FileType::from_extension(ext);

            if matches!(file_type, crate::database::FileType::Unknown) {
                continue;
            }

            // Trial limit is enforced inside IngestionEngine::ingest_file()
            match state
                .ingestion_engine
                .ingest_file(file_path, &app_handle)
                .await
            {
                Ok(doc) => documents.push(doc),
                Err(RecallError::TrialLimitReached(msg)) => {
                    tracing::warn!("Trial limit reached during directory ingest: {}", msg);
                    errors.push(msg);
                    break;
                }
                Err(e) => {
                    tracing::error!("Failed to ingest {:?}: {}", file_path, e);
                    errors.push(format!("{}: {}", file_path.display(), e));
                }
            }
        }
    }

    if documents.is_empty() && !errors.is_empty() {
        return Err(RecallError::Ingestion(format!(
            "All files failed to ingest: {}",
            errors.join("; ")
        )));
    }

    Ok(documents)
}

#[tauri::command]
pub async fn cancel_ingestion(
    state: State<'_, Arc<AppState>>,
    document_id: String,
) -> Result<(), RecallError> {
    state.ingestion_engine.cancel(&document_id);
    Ok(())
}

#[tauri::command]
pub async fn get_ingestion_progress(
    state: State<'_, Arc<AppState>>,
    document_id: Option<String>,
) -> Result<Vec<IngestionProgress>, RecallError> {
    if let Some(id) = document_id {
        Ok(state
            .ingestion_engine
            .get_progress(&id)
            .map(|p| vec![p])
            .unwrap_or_default())
    } else {
        Ok(state.ingestion_engine.get_all_progress())
    }
}

/// Re-ingest a document with updated extraction logic (e.g., to apply OCR)
#[tauri::command]
pub async fn reingest_document(
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
    id: String,
) -> Result<Document, RecallError> {
    // Get the document's file path
    let doc = state
        .database
        .get_document(&id)?
        .ok_or_else(|| RecallError::NotFound(format!("Document not found: {}", id)))?;

    let file_path = PathBuf::from(&doc.file_path);

    if !file_path.exists() {
        return Err(RecallError::NotFound(format!(
            "Original file no longer exists: {}",
            doc.file_path
        )));
    }

    // Delete the existing document (and its chunks/embeddings)
    state.database.delete_document(&id)?;

    // Re-ingest with updated code
    state.ingestion_engine.ingest_file(&file_path, &app_handle).await
}

/// Get the current ingestion queue status
#[tauri::command]
pub async fn get_ingestion_queue(
    state: State<'_, Arc<AppState>>,
) -> Result<IngestionQueueStatus, RecallError> {
    let (queue_len, is_processing) = state.ingestion_engine.get_queue_status();
    let queued_files = state.ingestion_engine.get_queued_files();

    Ok(IngestionQueueStatus {
        queue_length: queue_len,
        is_processing,
        queued_files,
    })
}

/// Status of the ingestion queue
#[derive(serde::Serialize, Clone)]
pub struct IngestionQueueStatus {
    pub queue_length: usize,
    pub is_processing: bool,
    pub queued_files: Vec<QueuedFile>,
}
