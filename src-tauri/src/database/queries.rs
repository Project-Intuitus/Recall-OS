use super::models::*;
use crate::error::{RecallError, Result};
use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row};
use std::path::Path;
use uuid::Uuid;

/// Normalize file path for consistent database lookups
/// Converts to absolute path and normalizes separators
fn normalize_path(path: &str) -> String {
    let path_obj = Path::new(path);

    // Try to canonicalize (resolve symlinks, normalize), fall back to string normalization
    match path_obj.canonicalize() {
        Ok(canonical) => {
            let s = canonical.to_string_lossy().to_string();
            // Strip Windows extended-length path prefix (\\?\) for consistency
            // canonicalize() on Windows adds this prefix, but paths are stored without it
            s.strip_prefix("\\\\?\\").unwrap_or(&s).to_string()
        }
        Err(_) => {
            // Fallback: just normalize separators
            path.replace('/', "\\").replace("\\\\", "\\")
        }
    }
}

impl super::Database {
    // Document queries
    pub fn insert_document(&self, doc: &Document) -> Result<()> {
        self.with_conn_mut(|conn| {
            // Use explicit transaction for reliable insertion
            let tx = conn.transaction()?;
            tx.execute(
                r#"
                INSERT INTO documents (id, title, file_path, file_type, file_size, file_hash, mime_type, status, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    doc.id,
                    doc.title,
                    doc.file_path,
                    doc.file_type.as_str(),
                    doc.file_size,
                    doc.file_hash,
                    doc.mime_type,
                    doc.status.as_str(),
                    doc.metadata.to_string(),
                ],
            )?;
            tx.commit()?;

            tracing::debug!("Document inserted and committed: {}", doc.id);
            Ok(())
        })
    }

    pub fn update_document_status(
        &self,
        id: &str,
        status: DocumentStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE documents
                SET status = ?, error_message = ?, updated_at = datetime('now'),
                    ingested_at = CASE WHEN ? = 'completed' THEN datetime('now') ELSE ingested_at END
                WHERE id = ?
                "#,
                params![status.as_str(), error_message, status.as_str(), id],
            )?;
            Ok(())
        })
    }

    pub fn get_document(&self, id: &str) -> Result<Option<Document>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, file_path, file_type, file_size, file_hash, mime_type,
                       created_at, updated_at, ingested_at, status, error_message, metadata
                FROM documents WHERE id = ?
                "#,
            )?;

            let doc = stmt.query_row([id], Self::row_to_document).optional()?;
            Ok(doc)
        })
    }

    pub fn get_document_by_path(&self, path: &str) -> Result<Option<Document>> {
        // Normalize path for consistent lookups (handle different path separators)
        let normalized_path = normalize_path(path);

        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, file_path, file_type, file_size, file_hash, mime_type,
                       created_at, updated_at, ingested_at, status, error_message, metadata
                FROM documents WHERE file_path = ?
                "#,
            )?;

            let doc = stmt.query_row([&normalized_path], Self::row_to_document).optional()?;
            Ok(doc)
        })
    }

    pub fn get_document_by_hash(&self, hash: &str) -> Result<Option<Document>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, file_path, file_type, file_size, file_hash, mime_type,
                       created_at, updated_at, ingested_at, status, error_message, metadata
                FROM documents WHERE file_hash = ?
                "#,
            )?;

            let doc = stmt.query_row([hash], Self::row_to_document).optional()?;
            Ok(doc)
        })
    }

    pub fn update_document_path(&self, id: &str, new_path: &str, new_title: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE documents
                SET file_path = ?, title = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![new_path, new_title, id],
            )?;
            Ok(())
        })
    }

    /// Update only the title of a document
    pub fn update_document_title(&self, id: &str, title: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE documents
                SET title = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![title, id],
            )?;
            Ok(())
        })
    }

    pub fn get_all_documents(&self) -> Result<Vec<Document>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, file_path, file_type, file_size, file_hash, mime_type,
                       created_at, updated_at, ingested_at, status, error_message, metadata
                FROM documents ORDER BY updated_at DESC
                "#,
            )?;

            let docs = stmt
                .query_map([], Self::row_to_document)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(docs)
        })
    }

    pub fn delete_document(&self, id: &str) -> Result<()> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Delete from vector table first - log warning if it fails
            match tx.execute(
                "DELETE FROM vec_chunks WHERE chunk_id IN (SELECT id FROM chunks WHERE document_id = ?)",
                [id]
            ) {
                Ok(count) => tracing::debug!("Deleted {} vector chunks for document {}", count, id),
                Err(e) => tracing::warn!("Failed to delete vector chunks for document {}: {}", id, e),
            }

            // Delete chunks explicitly
            tx.execute("DELETE FROM chunks WHERE document_id = ?", [id])?;

            // Delete the document
            tx.execute("DELETE FROM documents WHERE id = ?", [id])?;

            tx.commit()?;
            Ok(())
        })
    }

    fn row_to_document(row: &Row<'_>) -> rusqlite::Result<Document> {
        Ok(Document {
            id: row.get(0)?,
            title: row.get(1)?,
            file_path: row.get(2)?,
            file_type: row.get::<_, String>(3)?.parse().unwrap_or(FileType::Unknown),
            file_size: row.get(4)?,
            file_hash: row.get(5)?,
            mime_type: row.get(6)?,
            created_at: row
                .get::<_, String>(7)?
                .parse()
                .unwrap_or_else(|_| Utc::now()),
            updated_at: row
                .get::<_, String>(8)?
                .parse()
                .unwrap_or_else(|_| Utc::now()),
            ingested_at: row
                .get::<_, Option<String>>(9)?
                .and_then(|s| s.parse().ok()),
            status: row
                .get::<_, String>(10)?
                .parse()
                .unwrap_or(DocumentStatus::Pending),
            error_message: row.get(11)?,
            metadata: {
                let metadata_str: String = row.get(12)?;
                match metadata_str.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Failed to parse document metadata, using empty object: {}", e);
                        serde_json::json!({})
                    }
                }
            },
        })
    }

    // Chunk queries
    pub fn insert_chunk(&self, chunk: &Chunk) -> Result<i64> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO chunks (document_id, chunk_index, content, token_count, start_offset, end_offset, page_number, timestamp_start, timestamp_end, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    chunk.document_id,
                    chunk.chunk_index,
                    chunk.content,
                    chunk.token_count,
                    chunk.start_offset,
                    chunk.end_offset,
                    chunk.page_number,
                    chunk.timestamp_start,
                    chunk.timestamp_end,
                    chunk.metadata.to_string(),
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    pub fn insert_chunks(&self, chunks: &[Chunk]) -> Result<Vec<i64>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let document_id = &chunks[0].document_id;

        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Verify document exists INSIDE the transaction to avoid TOCTOU race
            let doc_exists: bool = tx.query_row(
                "SELECT 1 FROM documents WHERE id = ?",
                [document_id],
                |_| Ok(true),
            ).unwrap_or(false);

            if !doc_exists {
                return Err(RecallError::Other(format!(
                    "Cannot insert chunks: document {} does not exist",
                    document_id
                )));
            }

            let mut ids = Vec::with_capacity(chunks.len());

            for chunk in chunks {
                tx.execute(
                    r#"
                    INSERT INTO chunks (document_id, chunk_index, content, token_count, start_offset, end_offset, page_number, timestamp_start, timestamp_end, metadata)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        chunk.document_id,
                        chunk.chunk_index,
                        chunk.content,
                        chunk.token_count,
                        chunk.start_offset,
                        chunk.end_offset,
                        chunk.page_number,
                        chunk.timestamp_start,
                        chunk.timestamp_end,
                        chunk.metadata.to_string(),
                    ],
                )?;
                ids.push(tx.last_insert_rowid());
            }

            tx.commit()?;
            Ok(ids)
        })
    }

    pub fn get_chunks_for_document(&self, document_id: &str) -> Result<Vec<Chunk>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, document_id, chunk_index, content, token_count, start_offset, end_offset,
                       page_number, timestamp_start, timestamp_end, metadata, created_at
                FROM chunks WHERE document_id = ? ORDER BY chunk_index
                "#,
            )?;

            let chunks = stmt
                .query_map([document_id], Self::row_to_chunk)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(chunks)
        })
    }

    pub fn get_chunk(&self, id: i64) -> Result<Option<Chunk>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, document_id, chunk_index, content, token_count, start_offset, end_offset,
                       page_number, timestamp_start, timestamp_end, metadata, created_at
                FROM chunks WHERE id = ?
                "#,
            )?;

            let chunk = stmt.query_row([id], Self::row_to_chunk).optional()?;
            Ok(chunk)
        })
    }

    pub fn get_chunks_by_ids(&self, ids: &[i64]) -> Result<Vec<Chunk>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        self.with_conn(|conn| {
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                r#"
                SELECT id, document_id, chunk_index, content, token_count, start_offset, end_offset,
                       page_number, timestamp_start, timestamp_end, metadata, created_at
                FROM chunks WHERE id IN ({})
                "#,
                placeholders
            );

            let mut stmt = conn.prepare(&query)?;
            let chunks = stmt
                .query_map(rusqlite::params_from_iter(ids), Self::row_to_chunk)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(chunks)
        })
    }

    fn row_to_chunk(row: &Row<'_>) -> rusqlite::Result<Chunk> {
        Ok(Chunk {
            id: row.get(0)?,
            document_id: row.get(1)?,
            chunk_index: row.get(2)?,
            content: row.get(3)?,
            token_count: row.get(4)?,
            start_offset: row.get(5)?,
            end_offset: row.get(6)?,
            page_number: row.get(7)?,
            timestamp_start: row.get(8)?,
            timestamp_end: row.get(9)?,
            metadata: {
                let metadata_str: String = row.get(10)?;
                match metadata_str.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Failed to parse chunk metadata, using empty object: {}", e);
                        serde_json::json!({})
                    }
                }
            },
            created_at: row
                .get::<_, String>(11)?
                .parse()
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    // Vector operations
    pub fn insert_embedding(&self, chunk_id: i64, embedding: &[f32]) -> Result<()> {
        self.with_conn(|conn| {
            let embedding_blob = embedding
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<u8>>();

            conn.execute(
                "INSERT INTO vec_chunks(chunk_id, embedding) VALUES (?, vec_f32(?))",
                params![chunk_id, embedding_blob],
            )?;
            Ok(())
        })
    }

    pub fn insert_embeddings(&self, chunk_ids: &[i64], embeddings: &[Vec<f32>]) -> Result<()> {
        if chunk_ids.len() != embeddings.len() {
            return Err(RecallError::Other(
                "Mismatched chunk_ids and embeddings length".to_string(),
            ));
        }

        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            for (chunk_id, embedding) in chunk_ids.iter().zip(embeddings.iter()) {
                let embedding_blob = embedding
                    .iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect::<Vec<u8>>();

                tx.execute(
                    "INSERT INTO vec_chunks(chunk_id, embedding) VALUES (?, vec_f32(?))",
                    params![chunk_id, embedding_blob],
                )?;
            }

            tx.commit()?;
            Ok(())
        })
    }

    pub fn vector_search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<(i64, f64)>> {
        self.with_conn(|conn| {
            let embedding_blob = query_embedding
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<u8>>();

            // Note: sqlite-vec requires k=? constraint for KNN queries
            let mut stmt = conn.prepare(
                r#"
                SELECT chunk_id, distance
                FROM vec_chunks
                WHERE embedding MATCH ? AND k = ?
                ORDER BY distance
                "#,
            )?;

            let results = stmt
                .query_map(params![embedding_blob, k as i64], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(results)
        })
    }

    /// Find similar chunks to an existing chunk using its embedding
    pub fn vector_search_by_chunk(&self, chunk_id: i64, k: usize) -> Result<Vec<(i64, f64)>> {
        self.with_conn(|conn| {
            // First get the embedding for the source chunk
            let embedding: Option<Vec<u8>> = conn.query_row(
                "SELECT embedding FROM vec_chunks WHERE chunk_id = ?",
                params![chunk_id],
                |row| row.get(0),
            ).ok();

            let Some(embedding_blob) = embedding else {
                return Ok(vec![]);
            };

            // Search for similar chunks (excluding the source chunk)
            // Note: sqlite-vec requires k=? constraint for KNN queries
            let mut stmt = conn.prepare(
                r#"
                SELECT chunk_id, distance
                FROM vec_chunks
                WHERE embedding MATCH ? AND k = ?
                ORDER BY distance
                "#,
            )?;

            let results: Vec<(i64, f64)> = stmt
                .query_map(params![embedding_blob, k as i64], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
                })?
                .filter_map(|r| r.ok())
                .filter(|(id, _)| *id != chunk_id) // Exclude source chunk
                .collect();

            Ok(results)
        })
    }

    // Full-text search
    pub fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<(i64, f64)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT rowid, bm25(chunks_fts) as score
                FROM chunks_fts
                WHERE chunks_fts MATCH ?
                ORDER BY score
                LIMIT ?
                "#,
            )?;

            let results = stmt
                .query_map(params![query, limit as i64], |row| {
                    Ok((row.get::<_, i64>(0)?, -row.get::<_, f64>(1)?)) // Negate BM25 score (lower is better)
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(results)
        })
    }

    // Statistics
    pub fn get_ingestion_stats(&self) -> Result<IngestionStats> {
        self.with_conn(|conn| {
            let total_documents: i64 =
                conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;

            let completed_documents: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE status = 'completed'",
                [],
                |row| row.get(0),
            )?;

            let failed_documents: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )?;

            let pending_documents: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )?;

            let processing_documents: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE status = 'processing'",
                [],
                |row| row.get(0),
            )?;

            let total_chunks: i64 =
                conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;

            let total_size_bytes: i64 = conn.query_row(
                "SELECT COALESCE(SUM(file_size), 0) FROM documents",
                [],
                |row| row.get(0),
            )?;

            Ok(IngestionStats {
                total_documents,
                completed_documents,
                failed_documents,
                pending_documents,
                processing_documents,
                total_chunks,
                total_size_bytes,
            })
        })
    }

    // Conversations
    pub fn create_conversation(&self, title: Option<&str>) -> Result<Conversation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES (?, ?)",
                params![id, title],
            )?;
            Ok(())
        })?;

        Ok(Conversation {
            id,
            title: title.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn add_message(
        &self,
        conversation_id: &str,
        role: MessageRole,
        content: &str,
        citations: &[Citation],
    ) -> Result<Message> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let citations_json = serde_json::to_string(citations)?;

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, citations) VALUES (?, ?, ?, ?, ?)",
                params![
                    id,
                    conversation_id,
                    match role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "system",
                    },
                    content,
                    citations_json,
                ],
            )?;

            conn.execute(
                "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?",
                [conversation_id],
            )?;

            Ok(())
        })?;

        Ok(Message {
            id,
            conversation_id: conversation_id.to_string(),
            role,
            content: content.to_string(),
            citations: citations.to_vec(),
            created_at: now,
        })
    }

    pub fn get_conversation_messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, conversation_id, role, content, citations, created_at
                FROM messages WHERE conversation_id = ? ORDER BY created_at
                "#,
            )?;

            let messages = stmt
                .query_map([conversation_id], |row| {
                    let role_str: String = row.get(2)?;
                    let citations_str: String = row.get(4)?;

                    Ok(Message {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        role: match role_str.as_str() {
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            _ => MessageRole::System,
                        },
                        content: row.get(3)?,
                        citations: serde_json::from_str(&citations_str).unwrap_or_default(),
                        created_at: row
                            .get::<_, String>(5)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(messages)
        })
    }

    pub fn get_all_conversations(&self) -> Result<Vec<Conversation>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, created_at, updated_at
                FROM conversations ORDER BY updated_at DESC
                "#,
            )?;

            let conversations = stmt
                .query_map([], |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row
                            .get::<_, String>(2)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: row
                            .get::<_, String>(3)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(conversations)
        })
    }

    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, created_at, updated_at
                FROM conversations WHERE id = ?
                "#,
            )?;

            let conversation = stmt
                .query_row([id], |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row
                            .get::<_, String>(2)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: row
                            .get::<_, String>(3)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                    })
                })
                .optional()?;

            Ok(conversation)
        })
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Delete messages first (foreign key constraint)
            tx.execute("DELETE FROM messages WHERE conversation_id = ?", [id])?;

            // Delete the conversation
            tx.execute("DELETE FROM conversations WHERE id = ?", [id])?;

            tx.commit()?;
            Ok(())
        })
    }

    pub fn update_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE conversations
                SET title = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![title, id],
            )?;
            Ok(())
        })
    }

    pub fn update_document_metadata(&self, id: &str, metadata: serde_json::Value) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE documents
                SET metadata = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![metadata.to_string(), id],
            )?;
            Ok(())
        })
    }
}
