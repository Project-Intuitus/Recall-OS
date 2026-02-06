use crate::error::{RecallError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const EMBEDDING_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    content: EmbedContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<u32>,
}

#[derive(Debug, Serialize)]
struct EmbedContent {
    parts: Vec<EmbedPart>,
}

#[derive(Debug, Serialize)]
struct EmbedPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct BatchEmbedRequest {
    requests: Vec<EmbedContentRequest>,
}

#[derive(Debug, Serialize)]
struct EmbedContentRequest {
    model: String,
    content: EmbedContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: EmbeddingValues,
}

#[derive(Debug, Deserialize)]
struct BatchEmbedResponse {
    embeddings: Vec<EmbeddingValues>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[derive(Clone)]
pub struct EmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
}

impl EmbeddingClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "{}/{}:embedContent?key={}",
            EMBEDDING_API_URL, self.model, self.api_key
        );

        let request = EmbedRequest {
            model: format!("models/{}", self.model),
            content: EmbedContent {
                parts: vec![EmbedPart {
                    text: text.to_string(),
                }],
            },
            output_dimensionality: Some(768),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(RecallError::Embedding(format!(
                "API error {}: {}",
                status, response_text
            )));
        }

        let embed_response: EmbedResponse = serde_json::from_str(&response_text)
            .map_err(|e| RecallError::Embedding(format!(
                "Failed to parse response: {} - Body: {}",
                e,
                if response_text.len() > 200 { &response_text[..200] } else { &response_text }
            )))?;
        Ok(embed_response.embedding.values)
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Gemini API allows up to 100 texts per batch
        const BATCH_SIZE: usize = 100;
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(BATCH_SIZE) {
            let url = format!(
                "{}/{}:batchEmbedContents?key={}",
                EMBEDDING_API_URL, self.model, self.api_key
            );

            let requests: Vec<EmbedContentRequest> = chunk
                .iter()
                .map(|text| EmbedContentRequest {
                    model: format!("models/{}", self.model),
                    content: EmbedContent {
                        parts: vec![EmbedPart { text: text.clone() }],
                    },
                    output_dimensionality: Some(768),
                })
                .collect();

            let batch_request = BatchEmbedRequest { requests };

            let response = self
                .client
                .post(&url)
                .json(&batch_request)
                .send()
                .await?;

            let status = response.status();
            let response_text = response.text().await.unwrap_or_default();

            if !status.is_success() {
                return Err(RecallError::Embedding(format!(
                    "Batch API error {}: {}",
                    status, response_text
                )));
            }

            let batch_response: BatchEmbedResponse = serde_json::from_str(&response_text)
                .map_err(|e| RecallError::Embedding(format!(
                    "Failed to parse batch response: {} - Body: {}",
                    e,
                    if response_text.len() > 200 { &response_text[..200] } else { &response_text }
                )))?;

            for embedding in batch_response.embeddings {
                all_embeddings.push(embedding.values);
            }
        }

        Ok(all_embeddings)
    }
}
