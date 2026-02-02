use crate::error::RecallError;
use crate::rag::{RagQuery, RagResponse};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn query(
    state: State<'_, Arc<AppState>>,
    query: String,
    conversation_id: Option<String>,
) -> Result<RagResponse, RecallError> {
    let request = RagQuery {
        query,
        conversation_id,
        max_chunks: None,
        include_sources: false,
        document_ids: None,
    };

    state.rag_engine.query(request).await
}

#[tauri::command]
pub async fn query_with_sources(
    state: State<'_, Arc<AppState>>,
    query: String,
    conversation_id: Option<String>,
    max_chunks: Option<usize>,
    document_ids: Option<Vec<String>>,
) -> Result<RagResponse, RecallError> {
    let request = RagQuery {
        query,
        conversation_id,
        max_chunks,
        include_sources: true,
        document_ids,
    };

    state.rag_engine.query(request).await
}
