mod retriever;

pub use retriever::*;

use crate::database::{ChunkWithScore, Citation, Database, MessageRole, SearchType};
use crate::error::{RecallError, Result};
use crate::llm::{ContextChunk, ConversationMessage, GenerateRequest, LlmClient, LlmProvider};
use crate::state::Settings;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub struct RagEngine {
    database: Arc<Database>,
    llm_client: Arc<RwLock<Option<LlmClient>>>,
    settings: Arc<RwLock<Settings>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQuery {
    pub query: String,
    pub conversation_id: Option<String>,
    pub max_chunks: Option<usize>,
    pub include_sources: bool,
    pub document_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub sources: Vec<SourceChunk>,
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChunk {
    pub chunk_id: i64,
    pub document_id: String,
    pub document_title: String,
    pub content: String,
    pub page_number: Option<i32>,
    pub timestamp: Option<f64>,
    pub relevance_score: f64,
    pub search_type: SearchType,
}

impl RagEngine {
    pub fn new(
        database: Arc<Database>,
        llm_client: Arc<RwLock<Option<LlmClient>>>,
        settings: Arc<RwLock<Settings>>,
    ) -> Self {
        Self {
            database,
            llm_client,
            settings,
        }
    }

    pub async fn query(&self, request: RagQuery) -> Result<RagResponse> {
        // Clone LLM client to avoid holding lock across await
        let llm = {
            let guard = self.llm_client.read();
            guard
                .as_ref()
                .ok_or(RecallError::Config("LLM client not configured".to_string()))?
                .clone()
        };

        // Get or create conversation
        let (conversation_id, history) = match request.conversation_id {
            Some(id) => {
                // Fetch existing conversation history for context
                let messages = self.database.get_conversation_messages(&id)?;
                let history: Vec<ConversationMessage> = messages
                    .into_iter()
                    .map(|m| ConversationMessage {
                        role: match m.role {
                            MessageRole::User => "user".to_string(),
                            MessageRole::Assistant => "assistant".to_string(),
                            MessageRole::System => "system".to_string(),
                        },
                        content: m.content,
                    })
                    .collect();
                (id, history)
            }
            None => {
                let conv = self.database.create_conversation(Some(&request.query))?;
                (conv.id, vec![])
            }
        };

        // Retrieve relevant chunks using hybrid search
        let max_chunks = {
            let settings = self.settings.read();
            request.max_chunks.unwrap_or(settings.max_context_chunks)
        };

        let retriever = HybridRetriever::new(self.database.clone(), llm.clone());
        let chunks = retriever.retrieve(&request.query, max_chunks, request.document_ids.as_deref()).await?;

        if chunks.is_empty() {
            // No relevant context found
            return Ok(RagResponse {
                answer: "I don't have any relevant information in my knowledge base to answer this question. Please try adding relevant documents or rephrasing your question.".to_string(),
                citations: vec![],
                sources: vec![],
                conversation_id,
            });
        }

        // Build context for generation
        let source_chunks = self.build_source_chunks(&chunks)?;
        let context = self.build_context(&source_chunks);

        // Generate response
        let gen_request = GenerateRequest {
            prompt: request.query.clone(),
            system_prompt: Some(self.build_system_prompt()),
            context,
            history,
            max_tokens: Some(2000),
            temperature: Some(0.7),
        };

        let response = llm.generate(gen_request).await?;

        // Build citations from response
        let citations = self.build_citations(&response.citations, &source_chunks)?;

        // Save to conversation history
        self.database.add_message(
            &conversation_id,
            crate::database::MessageRole::User,
            &request.query,
            &[],
        )?;

        self.database.add_message(
            &conversation_id,
            crate::database::MessageRole::Assistant,
            &response.content,
            &citations,
        )?;

        Ok(RagResponse {
            answer: response.content,
            citations,
            sources: if request.include_sources {
                source_chunks
            } else {
                vec![]
            },
            conversation_id,
        })
    }

    fn build_source_chunks(&self, chunks: &[ChunkWithScore]) -> Result<Vec<SourceChunk>> {
        let mut sources = Vec::new();
        let mut doc_cache: HashMap<String, String> = HashMap::new();

        for cws in chunks {
            let doc_title = if let Some(title) = doc_cache.get(&cws.chunk.document_id) {
                title.clone()
            } else {
                let title = self
                    .database
                    .get_document(&cws.chunk.document_id)?
                    .map(|d| d.title)
                    .unwrap_or_else(|| "Unknown".to_string());
                doc_cache.insert(cws.chunk.document_id.clone(), title.clone());
                title
            };

            sources.push(SourceChunk {
                chunk_id: cws.chunk.id,
                document_id: cws.chunk.document_id.clone(),
                document_title: doc_title,
                content: cws.chunk.content.clone(),
                page_number: cws.chunk.page_number,
                timestamp: cws.chunk.timestamp_start,
                relevance_score: cws.score,
                search_type: cws.search_type,
            });
        }

        Ok(sources)
    }

    fn build_context(&self, sources: &[SourceChunk]) -> Vec<ContextChunk> {
        sources
            .iter()
            .map(|s| ContextChunk {
                id: s.chunk_id,
                content: s.content.clone(),
                source: s.document_title.clone(),
                page: s.page_number,
                timestamp: s.timestamp,
            })
            .collect()
    }

    fn build_system_prompt(&self) -> String {
        r#"You are RECALL.OS, an AI assistant that answers questions based on the user's personal knowledge base.

## Instructions

1. **Use Only Provided Context**: Answer questions using ONLY the information in the <context> section. Do not use external knowledge.

2. **Cite Your Sources**: When you use information from a chunk, cite it using [chunk_id] format. For example: "The project started in 2024 [123]."

3. **Be Honest About Limitations**: If the context doesn't contain enough information, say "I don't have detailed information about that in your knowledge base." Never claim you "cannot" do something - you CAN read all file types, but the content may not have been fully extracted.

4. **Preserve Details**: Include specific details, numbers, dates, and names from the context when relevant.

5. **Handle Timestamps**: For video/audio sources, mention timestamps when relevant so users can jump to that point.

6. **Handle Page Numbers**: For documents, reference page numbers when helpful for navigation.

7. **Use Your Knowledge for General Questions**: For general knowledge questions (like "what is coffee?"), you may use your training knowledge to provide helpful explanations, while noting that the user's knowledge base only contains what was indexed.

8. **Be Conversational**: Remember the conversation history and provide coherent follow-up responses.

## Response Format

Provide clear, well-organized answers. Use markdown formatting when appropriate:
- Use bullet points for lists
- Use headers for long answers with multiple sections
- Use code blocks for code or technical content

When citing sources, naturally integrate citations into your response."#.to_string()
    }

    fn build_citations(
        &self,
        citation_refs: &[crate::llm::CitationRef],
        sources: &[SourceChunk],
    ) -> Result<Vec<Citation>> {
        let source_map: HashMap<i64, &SourceChunk> =
            sources.iter().map(|s| (s.chunk_id, s)).collect();

        let citations = citation_refs
            .iter()
            .filter_map(|cr| {
                source_map.get(&cr.chunk_id).map(|source| Citation {
                    chunk_id: cr.chunk_id,
                    document_id: source.document_id.clone(),
                    document_title: source.document_title.clone(),
                    content_snippet: truncate_snippet(&source.content, 200),
                    page_number: source.page_number,
                    timestamp: source.timestamp,
                    relevance_score: source.relevance_score,
                })
            })
            .collect();

        Ok(citations)
    }
}

fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
