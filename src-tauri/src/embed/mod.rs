//! Embeddings. A single [`Embedder`] trait with two production paths — Ollama
//! (`nomic-embed-text`, default when detected on localhost) and, in the
//! frontend, a `@xenova/transformers` Web Worker bridged over IPC. Tests use a
//! deterministic [`MockEmbedder`] so the indexer and retrieval are exercised
//! without any model running.

pub mod ollama;

use async_trait::async_trait;

use crate::error::AppResult;

/// Produces vector embeddings for text. Implementations must return one vector
/// per input, each of length [`Embedder::dim`].
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Stable identifier of the model (e.g. `nomic-embed-text`). Persisted with
    /// each embedding so a model change is detectable and re-embed stays explicit.
    fn model_id(&self) -> &str;

    /// Dimensionality of the produced vectors.
    fn dim(&self) -> usize;

    /// Embed a batch of texts. Returns vectors in the same order as the input.
    async fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>>;
}

/// L2-normalize a vector in place. Cosine similarity over normalized vectors is
/// a plain dot product, which the vector store can index efficiently.
pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Deterministic, dependency-free embedder for tests and offline fallback.
/// Hashes token trigrams into a fixed-dimension bag-of-features vector, then
/// normalizes. Similar text → similar vectors, with zero external services.
pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl Default for MockEmbedder {
    fn default() -> Self {
        Self { dim: 64 }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    fn model_id(&self) -> &str {
        "mock-embed"
    }

    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0.0f32; self.dim];
                let lower = t.to_lowercase();
                for word in lower.split_whitespace() {
                    // Cheap stable hash (FNV-1a) into a bucket.
                    let mut h: u64 = 0xcbf29ce484222325;
                    for b in word.bytes() {
                        h ^= b as u64;
                        h = h.wrapping_mul(0x100000001b3);
                    }
                    let idx = (h as usize) % self.dim;
                    v[idx] += 1.0;
                }
                normalize(&mut v);
                v
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_makes_unit_length() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
        assert!((len - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_handles_zero_vector() {
        let mut v = vec![0.0, 0.0];
        normalize(&mut v);
        assert_eq!(v, vec![0.0, 0.0]);
    }

    #[tokio::test]
    async fn mock_embedder_is_deterministic_and_shaped() {
        let e = MockEmbedder::new(32);
        let a = e.embed(&["hello world".to_string()]).await.unwrap();
        let b = e.embed(&["hello world".to_string()]).await.unwrap();
        assert_eq!(a, b);
        assert_eq!(a[0].len(), 32);
    }

    #[tokio::test]
    async fn mock_embedder_similar_text_closer_than_dissimilar() {
        let e = MockEmbedder::new(128);
        let v = e
            .embed(&[
                "the cat sat on the mat".to_string(),
                "the cat sat on a mat".to_string(),
                "quarterly financial report figures".to_string(),
            ])
            .await
            .unwrap();
        let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        let close = dot(&v[0], &v[1]);
        let far = dot(&v[0], &v[2]);
        assert!(close > far, "similar={close} dissimilar={far}");
    }
}
