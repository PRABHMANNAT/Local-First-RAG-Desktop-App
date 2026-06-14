//! Typed query layer. This is the only place raw SQL for the domain tables
//! lives; ingest, retrieval, and commands call these functions. Keeping SQL
//! here keeps the rest of the core decoupled from the schema.

use sqlx::SqlitePool;

use crate::error::AppResult;

/// A chunk row joined with its document's path, as needed by retrieval.
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct ChunkRow {
    pub id: String,
    pub document_id: String,
    pub text: String,
    pub structural_path: Option<String>,
    pub locator: String,
    pub path_or_url: String,
}

/// Insert a source row.
pub async fn insert_source(
    pool: &SqlitePool,
    id: &str,
    kind: &str,
    uri: &str,
    status: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO source (id, kind, uri, status) VALUES (?1, ?2, ?3, ?4)")
        .bind(id)
        .bind(kind)
        .bind(uri)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update a source's status (`queued`/`ingesting`/`ready`/`error`/`stale`).
pub async fn set_source_status(pool: &SqlitePool, id: &str, status: &str) -> AppResult<()> {
    sqlx::query("UPDATE source SET status = ?2 WHERE id = ?1")
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

/// Look up an existing document by source + path, returning `(id, content_hash)`.
pub async fn document_by_path(
    pool: &SqlitePool,
    source_id: &str,
    path_or_url: &str,
) -> AppResult<Option<(String, String)>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id, content_hash FROM document WHERE source_id = ?1 AND path_or_url = ?2",
    )
    .bind(source_id)
    .bind(path_or_url)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Delete a document and (via cascade) its chunks and vectors.
pub async fn delete_document(pool: &SqlitePool, document_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM document WHERE id = ?1")
        .bind(document_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a document row.
#[allow(clippy::too_many_arguments)]
pub async fn insert_document(
    pool: &SqlitePool,
    id: &str,
    source_id: &str,
    path_or_url: &str,
    title: Option<&str>,
    mime: Option<&str>,
    byte_size: i64,
    content_hash: &str,
    ingested_at: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO document
            (id, source_id, path_or_url, title, mime, byte_size, content_hash, ingested_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(id)
    .bind(source_id)
    .bind(path_or_url)
    .bind(title)
    .bind(mime)
    .bind(byte_size)
    .bind(content_hash)
    .bind(ingested_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a chunk row. `locator` is the JSON-encoded [`crate::model::Locator`].
#[allow(clippy::too_many_arguments)]
pub async fn insert_chunk(
    pool: &SqlitePool,
    id: &str,
    document_id: &str,
    ordinal: i64,
    text: &str,
    token_count: i64,
    structural_path: Option<&str>,
    locator: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO chunk
            (id, document_id, ordinal, text, token_count, structural_path, locator)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(id)
    .bind(document_id)
    .bind(ordinal)
    .bind(text)
    .bind(token_count)
    .bind(structural_path)
    .bind(locator)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record embedding provenance (model + dim) for a chunk.
pub async fn insert_embedding(
    pool: &SqlitePool,
    chunk_id: &str,
    model_id: &str,
    dim: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO embedding (chunk_id, model_id, dim) VALUES (?1, ?2, ?3)
         ON CONFLICT(chunk_id) DO UPDATE SET model_id = excluded.model_id, dim = excluded.dim",
    )
    .bind(chunk_id)
    .bind(model_id)
    .bind(dim)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch chunk rows (with document path) by id, preserving the input order.
pub async fn chunks_by_ids(pool: &SqlitePool, ids: &[String]) -> AppResult<Vec<ChunkRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT c.id, c.document_id, c.text, c.structural_path, c.locator, d.path_or_url
         FROM chunk c JOIN document d ON d.id = c.document_id
         WHERE c.id IN ({placeholders})"
    );
    let mut q = sqlx::query_as::<_, ChunkRow>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await?;

    // Reorder to match the requested id order (SQL IN doesn't preserve it).
    let mut by_id: std::collections::HashMap<String, ChunkRow> =
        rows.into_iter().map(|r| (r.id.clone(), r)).collect();
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Fetch every chunk's `(id, text)` for building the in-process BM25 index.
/// Ordered by document then ordinal so the lexical index is built
/// deterministically (BM25 tie-breaks on insertion order).
pub async fn all_chunk_texts(pool: &SqlitePool) -> AppResult<Vec<(String, String)>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.id, c.text FROM chunk c
         JOIN document d ON d.id = c.document_id
         ORDER BY c.document_id, c.ordinal",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Insert a conversation row.
pub async fn insert_conversation(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    created_at: i64,
) -> AppResult<()> {
    sqlx::query("INSERT INTO conversation (id, title, created_at) VALUES (?1, ?2, ?3)")
        .bind(id)
        .bind(title)
        .bind(created_at)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a message row (`role` is `user`/`assistant`/`system`).
pub async fn insert_message(
    pool: &SqlitePool,
    id: &str,
    conversation_id: &str,
    role: &str,
    content: &str,
    created_at: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO message (id, conversation_id, role, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(id)
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record a citation linking a message to a chunk.
pub async fn insert_citation(
    pool: &SqlitePool,
    message_id: &str,
    chunk_id: &str,
    retrieved_score: f64,
    used_in_answer: bool,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO citation (message_id, chunk_id, retrieved_score, used_in_answer)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(message_id, chunk_id) DO UPDATE SET
             retrieved_score = excluded.retrieved_score,
             used_in_answer = excluded.used_in_answer",
    )
    .bind(message_id)
    .bind(chunk_id)
    .bind(retrieved_score)
    .bind(used_in_answer as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Count chunks in a workspace (test/debug helper).
pub async fn count_chunks(pool: &SqlitePool) -> AppResult<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunk")
        .fetch_one(pool)
        .await?;
    Ok(n)
}
