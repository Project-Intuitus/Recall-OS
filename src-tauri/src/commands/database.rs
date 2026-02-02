use crate::database::{Chunk, Document, IngestionStats};
use crate::error::RecallError;
use crate::llm::{GenerateRequest, LlmProvider};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCategory {
    pub category: String,
    pub confidence: f32,
}

const CONTENT_CATEGORIES: &[&str] = &[
    "Science & Technology",
    "Business & Finance",
    "Health & Medicine",
    "Education & Learning",
    "Arts & Entertainment",
    "News & Current Events",
    "Legal & Compliance",
    "Personal & Lifestyle",
    "Travel & Geography",
    "Food & Cooking",
    "Sports & Fitness",
    "History & Culture",
    "Politics & Government",
    "Environment & Nature",
    "Other",
];

#[tauri::command]
pub async fn get_documents(state: State<'_, Arc<AppState>>) -> Result<Vec<Document>, RecallError> {
    state.database.get_all_documents()
}

#[tauri::command]
pub async fn get_document(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<Document>, RecallError> {
    state.database.get_document(&id)
}

#[tauri::command]
pub async fn delete_document(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), RecallError> {
    state.database.delete_document(&id)
}

#[tauri::command]
pub async fn get_chunks_for_document(
    state: State<'_, Arc<AppState>>,
    document_id: String,
) -> Result<Vec<Chunk>, RecallError> {
    state.database.get_chunks_for_document(&document_id)
}

#[tauri::command]
pub async fn get_ingestion_stats(
    state: State<'_, Arc<AppState>>,
) -> Result<IngestionStats, RecallError> {
    state.database.get_ingestion_stats()
}

#[tauri::command]
pub async fn reset_database(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), RecallError> {
    // Stop the file watcher first
    state.stop_watcher();

    // Try SQL-based reset first, fall back to hard reset if corrupted
    let sql_result = state.database.with_conn_mut(|conn| {
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM vec_chunks", [])?;
        tx.execute("DELETE FROM chunks_fts", [])?;
        tx.execute("DELETE FROM chunks", [])?;
        tx.execute("DELETE FROM messages", [])?;
        tx.execute("DELETE FROM conversations", [])?;
        tx.execute("DELETE FROM documents", [])?;
        tx.commit()?;
        Ok(())
    });

    match sql_result {
        Ok(()) => {
            tracing::info!("Database reset via SQL DELETE successful");
        }
        Err(e) => {
            tracing::warn!("SQL reset failed ({}), performing hard reset...", e);
            // Database is corrupted, do a hard reset (delete files and recreate)
            state.database.hard_reset()?;
        }
    }

    // Clear watched folders from settings
    {
        let mut settings = state.settings.write();
        settings.watched_folders.clear();
        settings.auto_ingest_enabled = false;
    }
    state.save_settings()?;

    // Clear in-memory ingestion progress
    state.ingestion_engine.clear_all_progress();

    // Notify frontend to clear progress UI
    app_handle.emit("ingestion-progress-cleared", ()).ok();

    tracing::info!("Database and watched folders reset successfully - v4");
    Ok(())
}

#[tauri::command]
pub async fn open_file_in_default_app(path: String) -> Result<(), RecallError> {
    // Validate path exists and is a file to prevent command injection
    let path_ref = std::path::Path::new(&path);
    if !path_ref.exists() {
        return Err(RecallError::NotFound(format!("File not found: {}", path)));
    }
    if !path_ref.is_file() {
        return Err(RecallError::Other("Path is not a file".to_string()));
    }

    // Use opener crate for safe cross-platform file opening
    // This avoids shell command injection by using OS APIs directly
    opener::open(&path)
        .map_err(|e| RecallError::Other(format!("Failed to open file: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn categorize_document(
    state: State<'_, Arc<AppState>>,
    document_id: String,
) -> Result<ContentCategory, RecallError> {
    // Get the document
    let doc = state
        .database
        .get_document(&document_id)?
        .ok_or_else(|| RecallError::NotFound(format!("Document not found: {}", document_id)))?;

    // Get chunks for content sample
    let chunks = state.database.get_chunks_for_document(&document_id)?;

    if chunks.is_empty() {
        return Err(RecallError::Other("Document has no content to categorize".to_string()));
    }

    // Take first few chunks as a sample (limit to ~2000 chars)
    let mut content_sample = String::new();
    for chunk in chunks.iter().take(5) {
        content_sample.push_str(&chunk.content);
        content_sample.push_str("\n\n");
        if content_sample.len() > 2000 {
            break;
        }
    }

    // Get LLM client
    let llm = {
        let guard = state.llm_client.read();
        guard
            .as_ref()
            .ok_or(RecallError::Config("LLM client not configured".to_string()))?
            .clone()
    };

    // Build categorization prompt
    let categories_list = CONTENT_CATEGORIES.join("\n- ");
    let prompt = format!(
        r#"Analyze this document and categorize it into ONE of these categories:
- {}

Document title: {}
Content sample:
{}

Respond with ONLY the category name, nothing else."#,
        categories_list, doc.title, content_sample
    );

    let request = GenerateRequest {
        prompt,
        system_prompt: Some("You are a document categorization assistant. Respond with only the category name.".to_string()),
        context: vec![],
        history: vec![],
        max_tokens: Some(50),
        temperature: Some(0.1),
    };

    let response = llm.generate(request).await?;
    let category = response.content.trim().to_string();

    // Validate category
    let valid_category = CONTENT_CATEGORIES
        .iter()
        .find(|&&c| category.eq_ignore_ascii_case(c))
        .map(|&c| c.to_string())
        .unwrap_or_else(|| "Other".to_string());

    // Update document metadata
    let mut metadata = doc.metadata.clone();
    metadata["content_category"] = serde_json::json!(valid_category);
    state.database.update_document_metadata(&document_id, metadata)?;

    Ok(ContentCategory {
        category: valid_category,
        confidence: 1.0,
    })
}

#[tauri::command]
pub async fn categorize_all_documents(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<(String, String)>, RecallError> {
    let documents = state.database.get_all_documents()?;
    let mut results = Vec::new();

    for doc in documents {
        // Skip if already categorized
        if doc.metadata.get("content_category").is_some() {
            results.push((doc.id.clone(), doc.metadata["content_category"].as_str().unwrap_or("Other").to_string()));
            continue;
        }

        // Skip if no chunks
        let chunks = state.database.get_chunks_for_document(&doc.id)?;
        if chunks.is_empty() {
            continue;
        }

        // Take first few chunks as a sample
        let mut content_sample = String::new();
        for chunk in chunks.iter().take(5) {
            content_sample.push_str(&chunk.content);
            content_sample.push_str("\n\n");
            if content_sample.len() > 2000 {
                break;
            }
        }

        // Get LLM client
        let llm = {
            let guard = state.llm_client.read();
            match guard.as_ref() {
                Some(client) => client.clone(),
                None => continue,
            }
        };

        // Build categorization prompt
        let categories_list = CONTENT_CATEGORIES.join("\n- ");
        let prompt = format!(
            r#"Analyze this document and categorize it into ONE of these categories:
- {}

Document title: {}
Content sample:
{}

Respond with ONLY the category name, nothing else."#,
            categories_list, doc.title, content_sample
        );

        let request = GenerateRequest {
            prompt,
            system_prompt: Some("You are a document categorization assistant. Respond with only the category name.".to_string()),
            context: vec![],
            history: vec![],
            max_tokens: Some(50),
            temperature: Some(0.1),
        };

        match llm.generate(request).await {
            Ok(response) => {
                let category = response.content.trim().to_string();
                let valid_category = CONTENT_CATEGORIES
                    .iter()
                    .find(|&&c| category.eq_ignore_ascii_case(c))
                    .map(|&c| c.to_string())
                    .unwrap_or_else(|| "Other".to_string());

                // Update document metadata
                let mut metadata = doc.metadata.clone();
                metadata["content_category"] = serde_json::json!(valid_category);
                if let Err(e) = state.database.update_document_metadata(&doc.id, metadata) {
                    tracing::warn!("Failed to update metadata for {}: {}", doc.id, e);
                } else {
                    results.push((doc.id.clone(), valid_category));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to categorize document {}: {}", doc.id, e);
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_content_categories() -> Result<Vec<String>, RecallError> {
    Ok(CONTENT_CATEGORIES.iter().map(|&s| s.to_string()).collect())
}
