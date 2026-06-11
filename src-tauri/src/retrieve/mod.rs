//! Retrieval. M1 is naive: embed the query, vector-search the store for the
//! top-k chunks, and join back the chunk text + document path for display and
//! citation. Hybrid (BM25 + RRF) and reranking arrive at M2 behind this surface.

use sqlx::SqlitePool;

use crate::db::repo;
use crate::embed::Embedder;
use crate::error::AppResult;
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
            chunk_id: r.id,
            text: r.text,
            structural_path: r.structural_path,
            locator: r.locator,
            path_or_url: r.path_or_url,
        })
        .collect())
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
}
