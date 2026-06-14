//! Ingest pipeline: walk a folder, and for each file read → hash → (skip if
//! unchanged) → chunk → embed → persist (document, chunks, embeddings, vectors).
//!
//! Idempotent on content hash; per-document failures are isolated (a bad file
//! is skipped, the run continues). Progress is reported via a callback so this
//! stays free of Tauri — the command layer wraps it to emit IPC events.

use std::path::Path;

use sqlx::SqlitePool;

use crate::db::repo;
use crate::embed::Embedder;
use crate::error::AppResult;
use crate::index::{VectorRecord, VectorStore};
use crate::ingest::chunker::{chunk_document, ChunkConfig, TextKind};
use crate::ingest::sources::{folder, repo as git_repo, url};
use std::path::PathBuf;

/// Tally of what an ingest run did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IngestSummary {
    pub documents_ingested: usize,
    pub documents_skipped: usize,
    pub documents_failed: usize,
    pub chunks_created: usize,
}

/// Per-document progress, surfaced to the UI by the command layer.
#[derive(Debug, Clone)]
pub struct DocProgress {
    pub path: String,
    pub index: usize,
    pub total: usize,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn mime_for(kind: TextKind) -> &'static str {
    match kind {
        TextKind::Markdown => "text/markdown",
        TextKind::Code | TextKind::Plain => "text/plain",
    }
}

/// Ingest every supported file under `root` into the workspace `pool`, embedding
/// with `embedder` and indexing into `store`. `source_id` must already exist.
pub async fn ingest_folder(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    source_id: &str,
    root: &Path,
    cfg: &ChunkConfig,
    mut on_progress: impl FnMut(DocProgress),
) -> AppResult<IngestSummary> {
    let files = folder::walk(root, &folder::IncludeConfig::default());
    let total = files.len();
    let mut summary = IngestSummary::default();

    for (i, file) in files.iter().enumerate() {
        let path_str = file.path.to_string_lossy().to_string();
        on_progress(DocProgress {
            path: path_str.clone(),
            index: i,
            total,
        });

        // Per-document failure isolation: a read error skips this file only.
        let bytes = match std::fs::read(&file.path) {
            Ok(b) => b,
            Err(_) => {
                summary.documents_failed += 1;
                continue;
            }
        };
        let hash = folder::content_hash(&bytes);

        // Idempotency: unchanged content is skipped; changed content replaces
        // the old document (cascade clears its chunks and vectors).
        if let Some((existing_id, existing_hash)) =
            repo::document_by_path(pool, source_id, &path_str).await?
        {
            if existing_hash == hash {
                summary.documents_skipped += 1;
                continue;
            }
            repo::delete_document(pool, &existing_id).await?;
            store.delete_document(&existing_id).await?;
        }

        let text = String::from_utf8_lossy(&bytes).to_string();
        let chunks = chunk_document(&text, file.text_kind, cfg);
        if chunks.is_empty() {
            summary.documents_skipped += 1;
            continue;
        }

        let doc_id = uuid::Uuid::now_v7().to_string();
        let title = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string());
        repo::insert_document(
            pool,
            &doc_id,
            source_id,
            &path_str,
            title.as_deref(),
            Some(mime_for(file.text_kind)),
            bytes.len() as i64,
            &hash,
            now_ms(),
        )
        .await?;

        // One embedding batch per document.
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let vectors = embedder.embed(&texts).await?;

        let mut records = Vec::with_capacity(chunks.len());
        for (chunk, vector) in chunks.iter().zip(vectors.into_iter()) {
            let chunk_id = uuid::Uuid::now_v7().to_string();
            let locator_json = serde_json::to_string(&chunk.locator)?;
            repo::insert_chunk(
                pool,
                &chunk_id,
                &doc_id,
                chunk.ordinal as i64,
                &chunk.text,
                chunk.token_count as i64,
                chunk.structural_path.as_deref(),
                &locator_json,
            )
            .await?;
            repo::insert_embedding(pool, &chunk_id, embedder.model_id(), embedder.dim() as i64)
                .await?;
            records.push(VectorRecord {
                chunk_id,
                document_id: doc_id.clone(),
                vector,
            });
        }
        store.upsert(&records).await?;

        summary.documents_ingested += 1;
        summary.chunks_created += chunks.len();
    }

    Ok(summary)
}

/// Persist a single in-memory document (already-extracted text) under
/// `source_id`: hash → skip-if-unchanged → chunk → embed → persist. Shared by
/// the URL source and any future single-document source. Returns the run tally.
#[allow(clippy::too_many_arguments)]
async fn ingest_text_document(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    source_id: &str,
    path_or_url: &str,
    title: Option<&str>,
    text: &str,
    kind: TextKind,
    cfg: &ChunkConfig,
) -> AppResult<IngestSummary> {
    let mut summary = IngestSummary::default();
    let bytes = text.as_bytes();
    let hash = folder::content_hash(bytes);

    if let Some((existing_id, existing_hash)) =
        repo::document_by_path(pool, source_id, path_or_url).await?
    {
        if existing_hash == hash {
            summary.documents_skipped += 1;
            return Ok(summary);
        }
        repo::delete_document(pool, &existing_id).await?;
        store.delete_document(&existing_id).await?;
    }

    let chunks = chunk_document(text, kind, cfg);
    if chunks.is_empty() {
        summary.documents_skipped += 1;
        return Ok(summary);
    }

    let doc_id = uuid::Uuid::now_v7().to_string();
    repo::insert_document(
        pool,
        &doc_id,
        source_id,
        path_or_url,
        title,
        Some(mime_for(kind)),
        bytes.len() as i64,
        &hash,
        now_ms(),
    )
    .await?;

    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let vectors = embedder.embed(&texts).await?;
    let mut records = Vec::with_capacity(chunks.len());
    for (chunk, vector) in chunks.iter().zip(vectors.into_iter()) {
        let chunk_id = uuid::Uuid::now_v7().to_string();
        let locator_json = serde_json::to_string(&chunk.locator)?;
        repo::insert_chunk(
            pool,
            &chunk_id,
            &doc_id,
            chunk.ordinal as i64,
            &chunk.text,
            chunk.token_count as i64,
            chunk.structural_path.as_deref(),
            &locator_json,
        )
        .await?;
        repo::insert_embedding(pool, &chunk_id, embedder.model_id(), embedder.dim() as i64).await?;
        records.push(VectorRecord {
            chunk_id,
            document_id: doc_id.clone(),
            vector,
        });
    }
    store.upsert(&records).await?;
    summary.documents_ingested += 1;
    summary.chunks_created += chunks.len();
    Ok(summary)
}

/// Fetch a URL, extract readable text, and ingest it as one document. The page
/// title (when present) becomes the document title; the URL is the locator path.
pub async fn ingest_url(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    source_id: &str,
    page_url: &str,
    cfg: &ChunkConfig,
) -> AppResult<IngestSummary> {
    let page = url::fetch_and_extract(page_url).await?;
    ingest_text_document(
        pool,
        store,
        embedder,
        source_id,
        page_url,
        page.title.as_deref(),
        &page.text,
        TextKind::Plain,
        cfg,
    )
    .await
}

/// Shallow-clone (or sync) a Git remote into `cache_root`, then ingest its
/// working tree with the folder walk (honoring `.gitignore` + `.mnemosignore`).
#[allow(clippy::too_many_arguments)]
pub async fn ingest_repo(
    pool: &SqlitePool,
    store: &dyn VectorStore,
    embedder: &dyn Embedder,
    source_id: &str,
    git_url: &str,
    cache_root: &Path,
    cfg: &ChunkConfig,
    on_progress: impl FnMut(DocProgress),
) -> AppResult<IngestSummary> {
    let remote = git_repo::parse_git_url(git_url)?;
    let dest: PathBuf = git_repo::cache_dir(cache_root, &remote);
    let checkout = git_repo::clone_or_sync(&remote, &dest)?;
    ingest_folder(
        pool,
        store,
        embedder,
        source_id,
        &checkout,
        cfg,
        on_progress,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WORKSPACE_MIGRATOR;
    use crate::embed::MockEmbedder;
    use crate::index::sqlite_store::SqliteVectorStore;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use tempfile::tempdir;

    async fn workspace_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn ingest_then_reingest_is_idempotent() {
        let pool = workspace_pool().await;
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        repo::insert_source(&pool, "s1", "folder", "/x", "ingesting")
            .await
            .unwrap();

        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# Title\nHello world content.").unwrap();
        std::fs::write(dir.path().join("b.txt"), "Plain text body here.").unwrap();

        let cfg = ChunkConfig::default();
        let first = ingest_folder(&pool, &store, &embedder, "s1", dir.path(), &cfg, |_| {})
            .await
            .unwrap();
        assert_eq!(first.documents_ingested, 2);
        assert!(first.chunks_created >= 2);

        // Second run with unchanged content: everything skipped, no new chunks.
        let before = repo::count_chunks(&pool).await.unwrap();
        let second = ingest_folder(&pool, &store, &embedder, "s1", dir.path(), &cfg, |_| {})
            .await
            .unwrap();
        assert_eq!(second.documents_ingested, 0);
        assert_eq!(second.documents_skipped, 2);
        assert_eq!(repo::count_chunks(&pool).await.unwrap(), before);
    }

    #[tokio::test]
    async fn changed_file_is_reingested_not_duplicated() {
        let pool = workspace_pool().await;
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        repo::insert_source(&pool, "s1", "folder", "/x", "ingesting")
            .await
            .unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        std::fs::write(&path, "# One\nfirst version text.").unwrap();
        let cfg = ChunkConfig::default();
        ingest_folder(&pool, &store, &embedder, "s1", dir.path(), &cfg, |_| {})
            .await
            .unwrap();

        std::fs::write(&path, "# One\na completely different second version.").unwrap();
        let run = ingest_folder(&pool, &store, &embedder, "s1", dir.path(), &cfg, |_| {})
            .await
            .unwrap();
        assert_eq!(run.documents_ingested, 1);

        // Exactly one document remains for that path.
        let docs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(docs, 1);
    }

    #[tokio::test]
    async fn ingest_text_document_persists_and_is_idempotent() {
        let pool = workspace_pool().await;
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        repo::insert_source(&pool, "u1", "url", "https://x", "ingesting")
            .await
            .unwrap();
        let cfg = ChunkConfig::default();

        let first = ingest_text_document(
            &pool,
            &store,
            &embedder,
            "u1",
            "https://example.com/post",
            Some("Post"),
            "A web page about sourdough bread and proofing yeast at home.",
            TextKind::Plain,
            &cfg,
        )
        .await
        .unwrap();
        assert_eq!(first.documents_ingested, 1);
        assert!(first.chunks_created >= 1);

        // Same URL + same text → skipped, no duplicate document.
        let again = ingest_text_document(
            &pool,
            &store,
            &embedder,
            "u1",
            "https://example.com/post",
            Some("Post"),
            "A web page about sourdough bread and proofing yeast at home.",
            TextKind::Plain,
            &cfg,
        )
        .await
        .unwrap();
        assert_eq!(again.documents_ingested, 0);
        assert_eq!(again.documents_skipped, 1);
        let docs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(docs, 1);
    }

    #[tokio::test]
    async fn progress_callback_fires_per_file() {
        let pool = workspace_pool().await;
        let store = SqliteVectorStore::new(pool.clone());
        let embedder = MockEmbedder::default();
        repo::insert_source(&pool, "s1", "folder", "/x", "ingesting")
            .await
            .unwrap();
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\nalpha.").unwrap();
        std::fs::write(dir.path().join("b.md"), "# B\nbeta.").unwrap();

        let mut seen = 0;
        ingest_folder(
            &pool,
            &store,
            &embedder,
            "s1",
            dir.path(),
            &ChunkConfig::default(),
            |p| {
                assert_eq!(p.total, 2);
                seen += 1;
            },
        )
        .await
        .unwrap();
        assert_eq!(seen, 2);
    }
}
