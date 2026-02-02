use crate::database::ChunkWithScore;
use crate::error::RecallError;
use crate::rag::HybridRetriever;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub document_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunks: Vec<ChunkWithScore>,
    pub total: usize,
}

#[tauri::command]
pub async fn search_documents(
    state: State<'_, Arc<AppState>>,
    request: SearchRequest,
) -> Result<SearchResult, RecallError> {
    let limit = request.limit.unwrap_or(20);

    // Clone LLM client to avoid holding lock across await
    let llm = {
        let guard = state.llm_client.read();
        guard
            .as_ref()
            .ok_or(RecallError::Config("LLM client not configured".to_string()))?
            .clone()
    };

    let retriever = HybridRetriever::new(state.database.clone(), llm);
    let chunks = retriever.retrieve(&request.query, limit, request.document_ids.as_deref()).await?;

    let total = chunks.len();

    Ok(SearchResult { chunks, total })
}

#[tauri::command]
pub async fn hybrid_search(
    state: State<'_, Arc<AppState>>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<ChunkWithScore>, RecallError> {
    let limit = limit.unwrap_or(20);

    // Clone LLM client to avoid holding lock across await
    let llm = {
        let guard = state.llm_client.read();
        guard
            .as_ref()
            .ok_or(RecallError::Config("LLM client not configured".to_string()))?
            .clone()
    };

    let retriever = HybridRetriever::new(state.database.clone(), llm);
    retriever.retrieve(&query, limit, None).await
}
