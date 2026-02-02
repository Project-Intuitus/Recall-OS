use crate::database::{Chunk, ChunkWithScore, Database, SearchType};
use crate::error::Result;
use crate::llm::{LlmClient, LlmProvider};
use std::collections::HashMap;
use std::sync::Arc;

pub struct HybridRetriever {
    database: Arc<Database>,
    llm: LlmClient,
}

/// Related document found through similarity search
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelatedDocument {
    pub id: String,
    pub title: String,
    pub similarity: f64,
}

impl HybridRetriever {
    pub fn new(database: Arc<Database>, llm: LlmClient) -> Self {
        Self { database, llm }
    }

    /// Find documents similar to the given document
    /// Returns other documents that share similar content
    pub async fn find_related_documents(
        &self,
        document_id: &str,
        limit: usize,
        min_similarity: f64,
    ) -> Result<Vec<RelatedDocument>> {
        // Get chunks for the source document
        let source_chunks = self.database.get_chunks_for_document(document_id)?;
        tracing::debug!("find_related_documents: {} source chunks for doc {}", source_chunks.len(), document_id);

        if source_chunks.is_empty() {
            return Ok(vec![]);
        }

        // Get embeddings for source chunks and search for similar ones
        let mut doc_scores: HashMap<String, (String, f64)> = HashMap::new();

        // Use first few chunks to find related content (avoid using all for performance)
        let chunks_to_check = source_chunks.iter().take(3).collect::<Vec<_>>();

        for chunk in chunks_to_check {
            // Vector search using chunk's embedding
            tracing::debug!("Searching for similar chunks to chunk_id={}", chunk.id);
            match self.database.vector_search_by_chunk(chunk.id, limit * 2) {
                Ok(results) => {
                    tracing::debug!("Vector search returned {} results for chunk {}", results.len(), chunk.id);
                    for (chunk_id, distance) in results {
                        tracing::debug!("  Result: chunk_id={}, distance={}, similarity={}", chunk_id, distance, 1.0 / (1.0 + distance));
                        // Get the chunk to find its document
                        if let Ok(Some(related_chunk)) = self.database.get_chunk(chunk_id) {
                            // Skip chunks from the same document
                            if related_chunk.document_id == document_id {
                                continue;
                            }

                            let similarity = 1.0 / (1.0 + distance);
                            if similarity >= min_similarity {
                                // Get document info
                                if let Ok(Some(doc)) = self.database.get_document(&related_chunk.document_id) {
                                    let entry = doc_scores.entry(doc.id.clone()).or_insert((doc.title.clone(), 0.0));
                                    // Keep the highest similarity score
                                    if similarity > entry.1 {
                                        entry.1 = similarity;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Vector search failed for chunk {}: {}", chunk.id, e);
                }
            }
        }

        // Convert to RelatedDocument and sort by similarity
        let mut related: Vec<RelatedDocument> = doc_scores
            .into_iter()
            .map(|(id, (title, similarity))| RelatedDocument { id, title, similarity })
            .collect();

        related.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        related.truncate(limit);

        Ok(related)
    }

    pub async fn retrieve(&self, query: &str, limit: usize, document_ids: Option<&[String]>) -> Result<Vec<ChunkWithScore>> {
        // Perform both vector and FTS search in parallel
        let vector_results = self.vector_search(query, limit * 2).await;
        let fts_results = self.fts_search(query, limit * 2);

        // Merge results using reciprocal rank fusion
        let merged = self.reciprocal_rank_fusion(
            vector_results.unwrap_or_default(),
            fts_results.unwrap_or_default(),
            limit,
            document_ids,
        )?;

        Ok(merged)
    }

    async fn vector_search(&self, query: &str, limit: usize) -> Result<Vec<(i64, f64, SearchType)>> {
        // Generate query embedding
        let embeddings = self.llm.embed(&[query.to_string()]).await?;
        let query_embedding = embeddings.into_iter().next().unwrap_or_default();

        if query_embedding.is_empty() {
            return Ok(vec![]);
        }

        // Search vectors
        let results = self.database.vector_search(&query_embedding, limit)?;

        // Convert distance to similarity score (assuming cosine distance)
        // Lower distance = higher similarity
        let scored: Vec<_> = results
            .into_iter()
            .map(|(chunk_id, distance)| {
                let similarity = 1.0 / (1.0 + distance);
                (chunk_id, similarity, SearchType::Vector)
            })
            .collect();

        Ok(scored)
    }

    fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<(i64, f64, SearchType)>> {
        // Prepare query for FTS5 (escape special characters)
        let fts_query = prepare_fts_query(query);

        let results = self.database.fts_search(&fts_query, limit)?;

        // Normalize BM25 scores to 0-1 range
        let max_score = results.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);
        let scored: Vec<_> = results
            .into_iter()
            .map(|(chunk_id, score)| {
                let normalized = if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                };
                (chunk_id, normalized, SearchType::Fts)
            })
            .collect();

        Ok(scored)
    }

    fn reciprocal_rank_fusion(
        &self,
        vector_results: Vec<(i64, f64, SearchType)>,
        fts_results: Vec<(i64, f64, SearchType)>,
        limit: usize,
        document_ids: Option<&[String]>,
    ) -> Result<Vec<ChunkWithScore>> {
        const K: f64 = 60.0; // RRF constant

        let mut rrf_scores: HashMap<i64, f64> = HashMap::new();
        let mut search_types: HashMap<i64, SearchType> = HashMap::new();

        // Calculate RRF scores for vector results
        for (rank, (chunk_id, _, search_type)) in vector_results.iter().enumerate() {
            let score = 1.0 / (K + (rank + 1) as f64);
            *rrf_scores.entry(*chunk_id).or_insert(0.0) += score;
            search_types.insert(*chunk_id, *search_type);
        }

        // Calculate RRF scores for FTS results
        for (rank, (chunk_id, _, search_type)) in fts_results.iter().enumerate() {
            let score = 1.0 / (K + (rank + 1) as f64);
            *rrf_scores.entry(*chunk_id).or_insert(0.0) += score;

            // If chunk appears in both, mark as hybrid
            if search_types.contains_key(chunk_id) {
                search_types.insert(*chunk_id, SearchType::Hybrid);
            } else {
                search_types.insert(*chunk_id, *search_type);
            }
        }

        // Sort by RRF score
        let mut scored: Vec<_> = rrf_scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Fetch more than limit to account for filtering, then apply document filter
        let fetch_limit = if document_ids.is_some() { limit * 3 } else { limit };
        let top_ids: Vec<i64> = scored.iter().take(fetch_limit).map(|(id, _)| *id).collect();
        let chunks = self.database.get_chunks_by_ids(&top_ids)?;

        // Build result with scores
        let chunk_map: HashMap<i64, Chunk> = chunks.into_iter().map(|c| (c.id, c)).collect();

        let results: Vec<ChunkWithScore> = scored
            .into_iter()
            .take(fetch_limit)
            .filter_map(|(id, score)| {
                chunk_map.get(&id).and_then(|chunk| {
                    // Filter by document_ids if provided
                    if let Some(doc_ids) = document_ids {
                        if !doc_ids.contains(&chunk.document_id) {
                            return None;
                        }
                    }
                    Some(ChunkWithScore {
                        chunk: chunk.clone(),
                        score,
                        search_type: search_types.get(&id).copied().unwrap_or(SearchType::Vector),
                    })
                })
            })
            .take(limit)
            .collect();

        Ok(results)
    }
}

fn prepare_fts_query(query: &str) -> String {
    // FTS5 query syntax:
    // - Words are AND'd by default
    // - Use OR for alternatives
    // - Use " for phrases
    // - Escape special characters to prevent query syntax injection

    // First, escape FTS5 special characters
    let escape_fts_char = |c: char| -> Option<char> {
        match c {
            // Remove these characters as they have special meaning in FTS5
            '"' | '*' | '(' | ')' | ':' | '^' | '-' | '+' | 'N' if false => None,
            // Keep alphanumeric and safe punctuation
            _ if c.is_alphanumeric() || c.is_whitespace() || c == '\'' || c == '.' || c == ',' => Some(c),
            // Remove other special characters
            _ => Some(' '),
        }
    };

    let sanitized: String = query.chars().filter_map(escape_fts_char).collect();
    let sanitized = sanitized.trim();

    if sanitized.is_empty() {
        return String::new();
    }

    // Split into words and filter out empty strings
    let words: Vec<&str> = sanitized
        .split_whitespace()
        .filter(|w| !w.is_empty() && w.len() > 1)
        .collect();

    if words.is_empty() {
        return String::new();
    }

    if words.len() == 1 {
        // Single word: use prefix matching (safe since we've sanitized)
        format!("{}*", words[0])
    } else {
        // Multiple words: use phrase matching with escaped quotes
        // The sanitized query has no quotes, so this is safe
        let escaped_query = sanitized.replace('"', "");
        let phrase = format!("\"{}\"", escaped_query);
        let terms = words.iter().map(|w| format!("{}*", w)).collect::<Vec<_>>().join(" OR ");
        format!("{} OR {}", phrase, terms)
    }
}

pub struct TieredRetriever {
    database: Arc<Database>,
    llm: LlmClient,
}

impl TieredRetriever {
    pub fn new(database: Arc<Database>, llm: LlmClient) -> Self {
        Self { database, llm }
    }

    /// Fast retrieval: top 10 results for quick answers
    pub async fn retrieve_fast(&self, query: &str, document_ids: Option<&[String]>) -> Result<Vec<ChunkWithScore>> {
        let retriever = HybridRetriever::new(self.database.clone(), self.llm.clone());
        retriever.retrieve(query, 10, document_ids).await
    }

    /// Deep retrieval: top 50 results for comprehensive answers
    pub async fn retrieve_deep(&self, query: &str, document_ids: Option<&[String]>) -> Result<Vec<ChunkWithScore>> {
        let retriever = HybridRetriever::new(self.database.clone(), self.llm.clone());
        retriever.retrieve(query, 50, document_ids).await
    }
}
