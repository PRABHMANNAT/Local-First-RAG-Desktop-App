//! Retrieval. M1 was naive: embed the query, vector-search the store, join back
//! chunk text + document path. M2 adds the hybrid path — vector ∪ BM25 candidate
//! lists fused with Reciprocal Rank Fusion, near-duplicates dropped via MinHash,
//! then greedily packed to a token budget. Both paths share the same
//! [`RetrievedChunk`] surface so the answerer and citation UI are unaffected.

pub mod dedup;
pub mod fusion;
pub mod packing;

use sqlx::SqlitePool;

use crate::db::repo;
use crate::embed::Embedder;
use crate::error::AppResult;
use crate::index::bm25::Bm25Index;
use crate::index::VectorStore;

/// A retrieved chunk with everything the answerer and citation UI need.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub text: String,
    pub structural_path: Option<String>,
    /// JSON-encoded [`crate::model::Locator`].
    pub locator: String,
    pub path_or_url: String,
    pub score: f32,
    /// Approximate token cost of `text`, used by the budget packer.
    pub token_count: usize,
}

impl packing::Packable for RetrievedChunk {
    fn token_count(&self) -> usize {
        self.token_count
    }
}

/// Tuning knobs for hybrid retrieval. Defaults mirror PLAN.md §6.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Candidates pulled from each of the vector and BM25 arms before fusion.
    pub per_arm_k: usize,
    /// RRF damping constant.
    pub rrf_k: f32,
    /// Estimated-Jaccard threshold at/above which two chunks are near-dupes.
    pub dedup_threshold: f32,
    /// Token budget the packed result must fit within.
    pub token_budget: usize,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            per_arm_k: 30,
            rrf_k: fusion::DEFAULT_RRF_K,
            dedup_threshold: 0.85,
            token_budget: packing::DEFAULT_TOKEN_BUDGET,
        }
    }
}

/// Cheap token-count estimate: word count scaled by a typical tokens-per-word
/// ratio. Good enough for budgeting; the real tokenizer lives with the LLM.
fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    // ~1.3 tokens per whitespace word for English/code on common BPE vocabs.
    ((words as f32) * 1.3).ceil() as usize
}

/// Embed `query` and return the top-`k` most similar chunks, best first.
pub async fn retrieve(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
) -> AppResult<Vec<RetrievedChunk>> {
    let query_vec = embedder
        .embed(&[query.to_string()])
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

    let hits = store.search(&query_vec, k).await?;
    let ids: Vec<String> = hits.iter().map(|h| h.chunk_id.clone()).collect();
    let rows = repo::chunks_by_ids(pool, &ids).await?;

    let score_by_id: std::collections::HashMap<&str, f32> = hits
        .iter()
        .map(|h| (h.chunk_id.as_str(), h.score))
        .collect();

    Ok(rows
        .into_iter()
        .map(|r| RetrievedChunk {
            score: score_by_id.get(r.id.as_str()).copied().unwrap_or(0.0),
            token_count: estimate_tokens(&r.text),
            chunk_id: r.id,
            text: r.text,
            structural_path: r.structural_path,
            locator: r.locator,
            path_or_url: r.path_or_url,
        })
        .collect())
}

/// Hybrid retrieval: fuse vector and BM25 candidate lists, drop near-duplicates,
/// and pack to a token budget. The BM25 index is built on the fly from the
/// workspace's chunk corpus — fine at M2 sizes; a persistent index lands with
/// the LanceDB/tantivy swap.
///
/// Returns chunks best-first by fused rank (the `score` field carries the RRF
/// score, not comparable to cosine), already de-duplicated and budget-packed.
pub async fn retrieve_hybrid(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    query: &str,
    cfg: &HybridConfig,
) -> AppResult<Vec<RetrievedChunk>> {
    // --- Vector arm --------------------------------------------------------
    let query_vec = embedder
        .embed(&[query.to_string()])
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();
    let vec_hits = store.search(&query_vec, cfg.per_arm_k).await?;

    // --- BM25 arm ----------------------------------------------------------
    // Build the lexical index over the full chunk corpus, then query it.
    let corpus = repo::all_chunk_texts(pool).await?;
    let mut bm25 = Bm25Index::new();
    for (id, text) in &corpus {
        bm25.add(id.clone(), text);
    }
    let bm25_hits = bm25.search(query, cfg.per_arm_k);

    // --- Fuse --------------------------------------------------------------
    let fused = fusion::reciprocal_rank_fusion(&[vec_hits, bm25_hits], cfg.rrf_k);
    if fused.is_empty() {
        return Ok(Vec::new());
    }

    // Hydrate fused ids (fusion order preserved by chunks_by_ids).
    let ids: Vec<String> = fused.iter().map(|h| h.chunk_id.clone()).collect();
    let rows = repo::chunks_by_ids(pool, &ids).await?;
    let score_by_id: std::collections::HashMap<&str, f32> = fused
        .iter()
        .map(|h| (h.chunk_id.as_str(), h.score))
        .collect();
    let ranked: Vec<RetrievedChunk> = rows
        .into_iter()
        .map(|r| RetrievedChunk {
            score: score_by_id.get(r.id.as_str()).copied().unwrap_or(0.0),
            token_count: estimate_tokens(&r.text),
            chunk_id: r.id,
            text: r.text,
            structural_path: r.structural_path,
            locator: r.locator,
            path_or_url: r.path_or_url,
        })
        .collect();

    // --- Dedup (keep highest-ranked representative) ------------------------
    let texts: Vec<&str> = ranked.iter().map(|c| c.text.as_str()).collect();
    let keep = dedup::dedupe_indices(&texts, cfg.dedup_threshold);
    let deduped: Vec<RetrievedChunk> = keep.into_iter().map(|i| ranked[i].clone()).collect();

    // --- Pack to budget ----------------------------------------------------
    Ok(packing::pack(deduped, cfg.token_budget))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{repo, WORKSPACE_MIGRATOR};
    use crate::embed::MockEmbedder;
    use crate::index::sqlite_store::SqliteVectorStore;
    use crate::ingest::chunker::ChunkConfig;
    use crate::ingest::pipeline::ingest_folder;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use tempfile::tempdir;

    #[tokio::test]
    async fn retrieves_the_topically_relevant_document() {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        repo::insert_source(&pool, "s1", "folder", "/x", "ingesting")
            .await
            .unwrap();

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("cooking.md"),
            "# Cooking\nKnead the dough, proof the yeast, then bake the sourdough bread loaf.",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("finance.md"),
            "# Finance\nThe quarterly revenue report shows profit margins and tax depreciation.",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("space.md"),
            "# Space\nThe rocket engine ignites liquid oxygen propellant to reach orbit velocity.",
        )
        .unwrap();

        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::new(256);
        ingest_folder(
            &pool,
            &store,
            &embedder,
            "s1",
            dir.path(),
            &ChunkConfig::default(),
            |_| {},
        )
        .await
        .unwrap();

        let hits = retrieve(
            &pool,
            &store,
            &embedder,
            "sourdough bread dough yeast bake",
            3,
        )
        .await
        .unwrap();
        assert!(!hits.is_empty());
        assert!(
            hits[0].path_or_url.ends_with("cooking.md"),
            "expected cooking.md on top, got {}",
            hits[0].path_or_url
        );
        // Scores are sorted descending.
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[tokio::test]
    async fn retrieve_on_empty_store_returns_nothing() {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        let hits = retrieve(&pool, &store, &embedder, "anything", 5)
            .await
            .unwrap();
        assert!(hits.is_empty());
    }

    /// Shared fixture: three topically distinct docs ingested into a fresh
    /// in-memory workspace. Returns the wired pool/store/embedder.
    async fn fixture() -> (sqlx::SqlitePool, SqliteVectorStore, MockEmbedder) {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        repo::insert_source(&pool, "s1", "folder", "/x", "ingesting")
            .await
            .unwrap();
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("cooking.md"),
            "# Cooking\nKnead the dough, proof the yeast, then bake the sourdough bread loaf.",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("finance.md"),
            "# Finance\nThe quarterly revenue report shows profit margins and tax depreciation.",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("space.md"),
            "# Space\nThe rocket engine ignites liquid oxygen propellant to reach orbit velocity.",
        )
        .unwrap();
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::new(256);
        ingest_folder(
            &pool,
            &store,
            &embedder,
            "s1",
            dir.path(),
            &ChunkConfig::default(),
            |_| {},
        )
        .await
        .unwrap();
        // Keep `dir` alive for the test body via leak — the temp files are only
        // read during ingest above, but leaking avoids an early drop race.
        std::mem::forget(dir);
        (pool, store, embedder)
    }

    #[tokio::test]
    async fn hybrid_surfaces_the_lexically_exact_document() {
        let (pool, store, embedder) = fixture().await;
        // A query with rare, exact lexical terms — BM25's strength.
        let hits = retrieve_hybrid(
            &pool,
            &store,
            &embedder,
            "liquid oxygen propellant orbit velocity",
            &HybridConfig::default(),
        )
        .await
        .unwrap();
        assert!(!hits.is_empty());
        assert!(
            hits[0].path_or_url.ends_with("space.md"),
            "expected space.md on top, got {}",
            hits[0].path_or_url
        );
    }

    #[tokio::test]
    async fn hybrid_respects_the_token_budget() {
        let (pool, store, embedder) = fixture().await;
        let cfg = HybridConfig {
            token_budget: 5, // tiny — admits at most one short chunk
            ..HybridConfig::default()
        };
        let hits = retrieve_hybrid(&pool, &store, &embedder, "sourdough bread", &cfg)
            .await
            .unwrap();
        let total: usize = hits.iter().map(|h| h.token_count).sum();
        assert!(total <= 5, "packed {total} tokens, over budget");
    }

    #[tokio::test]
    async fn hybrid_on_empty_store_returns_nothing() {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        let hits = retrieve_hybrid(&pool, &store, &embedder, "x", &HybridConfig::default())
            .await
            .unwrap();
        assert!(hits.is_empty());
    }
}
