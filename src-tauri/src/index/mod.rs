//! Vector index. The [`VectorStore`] trait is the seam between retrieval and
//! whatever holds the vectors. M1 ships [`sqlite_store::SqliteVectorStore`], a
//! brute-force cosine search over vectors stored in SQLite — correct and fully
//! testable. A LanceDB-backed implementation drops in behind this trait at M2.

pub mod sqlite_store;

use async_trait::async_trait;

use crate::error::AppResult;

/// A vector to index, keyed by chunk and owning document.
#[derive(Debug, Clone)]
pub struct VectorRecord {
    pub chunk_id: String,
    pub document_id: String,
    pub vector: Vec<f32>,
}

/// A search hit: a chunk id and its similarity score (higher is closer).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredChunk {
    pub chunk_id: String,
    pub score: f32,
}

/// Storage and nearest-neighbor search over chunk embeddings.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert or replace vectors. Idempotent on `chunk_id`.
    async fn upsert(&self, records: &[VectorRecord]) -> AppResult<()>;

    /// Return the top-`k` chunks most similar to `query`, best first.
    async fn search(&self, query: &[f32], k: usize) -> AppResult<Vec<ScoredChunk>>;

    /// Drop all vectors belonging to a document (used on re-ingest).
    async fn delete_document(&self, document_id: &str) -> AppResult<()>;
}

/// Cosine similarity. For L2-normalized inputs this equals the dot product; we
/// compute the full form so it's correct for un-normalized vectors too.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom > f32::EPSILON {
        dot / denom
    } else {
        0.0
    }
}

/// Encode a vector as little-endian f32 bytes for BLOB storage.
pub fn encode_vector(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    bytes
}

/// Decode little-endian f32 bytes back into a vector. Returns an empty vector
/// if the byte length isn't a multiple of 4.
pub fn decode_vector(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() % 4 != 0 {
        return Vec::new();
    }
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_of_identical_is_one() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_orthogonal_is_zero() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths_is_zero() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn vector_round_trips_through_bytes() {
        let v = vec![0.5, -1.25, 3.0, 0.0];
        assert_eq!(decode_vector(&encode_vector(&v)), v);
    }

    #[test]
    fn decode_rejects_ragged_bytes() {
        assert!(decode_vector(&[1, 2, 3]).is_empty());
    }
}
