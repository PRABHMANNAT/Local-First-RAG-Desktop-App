//! Brute-force vector store over SQLite. Loads candidate vectors and ranks them
//! by cosine similarity in process. Correct and simple; fine for M1 workspaces.
//! Replaced/augmented by the LanceDB adapter at M2 for large corpora.

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::index::{
    cosine_similarity, decode_vector, encode_vector, ScoredChunk, VectorRecord, VectorStore,
};

/// A [`VectorStore`] backed by the workspace SQLite database.
pub struct SqliteVectorStore {
    pool: SqlitePool,
}

impl SqliteVectorStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VectorStore for SqliteVectorStore {
    async fn upsert(&self, records: &[VectorRecord]) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        for r in records {
            let data = encode_vector(&r.vector);
            sqlx::query(
                "INSERT INTO chunk_vector (chunk_id, document_id, dim, data)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(chunk_id) DO UPDATE SET
                     document_id = excluded.document_id,
                     dim = excluded.dim,
                     data = excluded.data",
            )
            .bind(&r.chunk_id)
            .bind(&r.document_id)
            .bind(r.vector.len() as i64)
            .bind(data)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn search(&self, query: &[f32], k: usize) -> AppResult<Vec<ScoredChunk>> {
        let rows: Vec<(String, Vec<u8>)> =
            sqlx::query_as("SELECT chunk_id, data FROM chunk_vector")
                .fetch_all(&self.pool)
                .await?;

        let mut scored: Vec<ScoredChunk> = rows
            .into_iter()
            .map(|(chunk_id, data)| ScoredChunk {
                chunk_id,
                score: cosine_similarity(query, &decode_vector(&data)),
            })
            .collect();

        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(k);
        Ok(scored)
    }

    async fn delete_document(&self, document_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM chunk_vector WHERE document_id = ?1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WORKSPACE_MIGRATOR;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::str::FromStr;

    async fn setup() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        // FK parents: one source, one document, three chunks.
        sqlx::query(
            "INSERT INTO source (id, kind, uri, status) VALUES ('s','folder','/x','ready')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO document (id, source_id, path_or_url, content_hash) VALUES ('d','s','/x/a.md','h')")
            .execute(&pool)
            .await
            .unwrap();
        for id in ["c1", "c2", "c3"] {
            sqlx::query("INSERT INTO chunk (id, document_id, ordinal, text, token_count, locator) VALUES (?1,'d',0,'t',1,'{}')")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        pool
    }

    fn rec(chunk_id: &str, v: Vec<f32>) -> VectorRecord {
        VectorRecord {
            chunk_id: chunk_id.to_string(),
            document_id: "d".to_string(),
            vector: v,
        }
    }

    #[tokio::test]
    async fn search_ranks_by_cosine_similarity() {
        let store = SqliteVectorStore::new(setup().await);
        store
            .upsert(&[
                rec("c1", vec![1.0, 0.0, 0.0]),
                rec("c2", vec![0.0, 1.0, 0.0]),
                rec("c3", vec![0.9, 0.1, 0.0]),
            ])
            .await
            .unwrap();

        let hits = store.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].chunk_id, "c1");
        assert_eq!(hits[1].chunk_id, "c3");
    }

    #[tokio::test]
    async fn upsert_is_idempotent_on_chunk_id() {
        let store = SqliteVectorStore::new(setup().await);
        store.upsert(&[rec("c1", vec![1.0, 0.0])]).await.unwrap();
        store.upsert(&[rec("c1", vec![0.0, 1.0])]).await.unwrap();
        let hits = store.search(&[0.0, 1.0], 5).await.unwrap();
        assert_eq!(hits.len(), 1, "re-upsert should replace, not duplicate");
        assert!((hits[0].score - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn delete_document_removes_its_vectors() {
        let store = SqliteVectorStore::new(setup().await);
        store
            .upsert(&[rec("c1", vec![1.0, 0.0]), rec("c2", vec![0.0, 1.0])])
            .await
            .unwrap();
        store.delete_document("d").await.unwrap();
        assert!(store.search(&[1.0, 0.0], 5).await.unwrap().is_empty());
    }
}
