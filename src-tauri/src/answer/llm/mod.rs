//! LLM clients for answer generation. The [`Llm`] trait seams the answerer from
//! the backend; M1 uses Ollama's `/api/generate`. A [`MockLlm`] gives tests a
//! deterministic answer (echoing a citation) with no model running.

pub mod ollama;

use async_trait::async_trait;

use crate::error::AppResult;

/// Generates an answer from a system prompt + user prompt.
#[async_trait]
pub trait Llm: Send + Sync {
    fn model_id(&self) -> &str;

    /// Produce the full answer text. (Token streaming to the UI is layered on
    /// top of this in a later pass; M1 returns the completed answer.)
    async fn generate(&self, system: &str, prompt: &str) -> AppResult<String>;
}

/// Deterministic LLM for tests: returns a fixed sentence and, if given, cites
/// the first valid chunk id so the citation-persistence path can be exercised.
pub struct MockLlm {
    cite: Option<String>,
}

impl MockLlm {
    pub fn new(cite_first: Option<String>) -> Self {
        Self { cite: cite_first }
    }
}

#[async_trait]
impl Llm for MockLlm {
    fn model_id(&self) -> &str {
        "mock-llm"
    }

    async fn generate(&self, _system: &str, _prompt: &str) -> AppResult<String> {
        Ok(match &self.cite {
            Some(id) => format!("This is a grounded answer [^{id}]."),
            None => "This is a grounded answer.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_llm_emits_citation_when_configured() {
        let llm = MockLlm::new(Some("c1".to_string()));
        let out = llm.generate("sys", "prompt").await.unwrap();
        assert!(out.contains("[^c1]"));
    }

    #[tokio::test]
    async fn mock_llm_without_citation() {
        let llm = MockLlm::new(None);
        let out = llm.generate("sys", "prompt").await.unwrap();
        assert!(!out.contains("[^"));
    }
}
