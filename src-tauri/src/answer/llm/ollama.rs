//! Ollama chat/generate client. Uses `/api/generate` with `stream: false` for
//! M1; the default model is selectable from the installed list in Settings.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::answer::llm::Llm;
use crate::error::{AppError, AppResult};

pub const DEFAULT_MODEL: &str = "llama3.1:8b-instruct-q4_0";

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    system: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

/// LLM backed by a local Ollama daemon.
pub struct OllamaLlm {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaLlm {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    pub fn default_local() -> Self {
        Self::new(crate::embed::ollama::DEFAULT_BASE_URL, DEFAULT_MODEL)
    }
}

#[async_trait]
impl Llm for OllamaLlm {
    fn model_id(&self) -> &str {
        &self.model
    }

    async fn generate(&self, system: &str, prompt: &str) -> AppResult<String> {
        let url = format!("{}/api/generate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&GenerateRequest {
                model: &self.model,
                system,
                prompt,
                stream: false,
            })
            .send()
            .await
            .map_err(|e| AppError::Other(format!("ollama generate request: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Other(format!(
                "ollama generate failed: HTTP {}",
                resp.status()
            )));
        }

        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Other(format!("ollama generate decode: {e}")))?;
        Ok(parsed.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shape() {
        let json = serde_json::to_value(GenerateRequest {
            model: "llama3.1",
            system: "s",
            prompt: "p",
            stream: false,
        })
        .unwrap();
        assert_eq!(json["model"], "llama3.1");
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn response_parses() {
        let parsed: GenerateResponse =
            serde_json::from_str(r#"{"response":"hello","done":true}"#).unwrap();
        assert_eq!(parsed.response, "hello");
    }
}
