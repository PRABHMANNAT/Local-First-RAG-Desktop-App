//! Ollama embedder. Talks to a local Ollama daemon's `/api/embeddings`
//! endpoint. Default model `nomic-embed-text` (768-dim). Used automatically when
//! Ollama is detected on localhost; otherwise the frontend xenova worker fills in.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::embed::{normalize, Embedder};
use crate::error::{AppError, AppResult};

/// Default Ollama endpoint and embedding model.
pub const DEFAULT_BASE_URL: &str = "http://localhost:11434";
pub const DEFAULT_MODEL: &str = "nomic-embed-text";
/// `nomic-embed-text` produces 768-dimensional vectors.
pub const DEFAULT_DIM: usize = 768;

#[derive(Debug, Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
}

/// Embedder backed by a local Ollama daemon.
pub struct OllamaEmbedder {
    client: reqwest::Client,
    base_url: String,
    model: String,
    dim: usize,
}

impl OllamaEmbedder {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dim: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            dim,
        }
    }

    /// Construct with the default localhost endpoint and `nomic-embed-text`.
    pub fn default_local() -> Self {
        Self::new(DEFAULT_BASE_URL, DEFAULT_MODEL, DEFAULT_DIM)
    }

    /// Return true if an Ollama daemon answers at `base_url` (GET `/api/tags`).
    pub async fn detect(base_url: &str) -> bool {
        let client = reqwest::Client::new();
        client
            .get(format!("{base_url}/api/tags"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    fn model_id(&self) -> &str {
        &self.model
    }

    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        let url = format!("{}/api/embeddings", self.base_url);
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let resp = self
                .client
                .post(&url)
                .json(&EmbeddingsRequest {
                    model: &self.model,
                    prompt: text,
                })
                .send()
                .await
                .map_err(|e| AppError::Other(format!("ollama embed request: {e}")))?;

            if !resp.status().is_success() {
                return Err(AppError::Other(format!(
                    "ollama embed failed: HTTP {}",
                    resp.status()
                )));
            }

            let mut parsed: EmbeddingsResponse = resp
                .json()
                .await
                .map_err(|e| AppError::Other(format!("ollama embed decode: {e}")))?;
            normalize(&mut parsed.embedding);
            out.push(parsed.embedding);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_model_and_prompt() {
        let json = serde_json::to_value(EmbeddingsRequest {
            model: "nomic-embed-text",
            prompt: "hi",
        })
        .unwrap();
        assert_eq!(json["model"], "nomic-embed-text");
        assert_eq!(json["prompt"], "hi");
    }

    #[test]
    fn response_parses_embedding_vector() {
        let body = r#"{"embedding":[0.1,0.2,0.3]}"#;
        let parsed: EmbeddingsResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.embedding, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn defaults_are_wired() {
        let e = OllamaEmbedder::default_local();
        assert_eq!(e.model_id(), "nomic-embed-text");
        assert_eq!(e.dim(), 768);
    }
}
