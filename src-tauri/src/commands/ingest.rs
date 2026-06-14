//! Ingest commands. Adding a folder source kicks off ingestion on a background
//! task and streams progress to the UI via events.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db::repo;
use crate::error::AppResult;
use crate::ingest::chunker::ChunkConfig;
use crate::ingest::pipeline::{ingest_folder, ingest_repo, ingest_url, DocProgress, IngestSummary};
use crate::state::AppState;

/// Event channel names for ingest, mirrored on the frontend.
pub const INGEST_PROGRESS: &str = "ingest:progress";
pub const INGEST_DONE: &str = "ingest:done";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestProgress {
    source_id: String,
    path: String,
    index: usize,
    total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestDone {
    source_id: String,
    ok: bool,
    documents: usize,
    chunks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Add a folder as a source and ingest it. Returns the new source id
/// immediately; ingestion runs in the background, emitting `ingest:progress`
/// per file and `ingest:done` on completion.
#[tauri::command]
pub async fn add_folder_source(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<String> {
    let ws = state.require_active().await?;
    let source_id = uuid::Uuid::now_v7().to_string();
    repo::insert_source(&ws.pool, &source_id, "folder", &path, "ingesting").await?;

    let root = PathBuf::from(&path);
    let sid = source_id.clone();
    tauri::async_runtime::spawn(async move {
        let progress_app = app.clone();
        let progress_sid = sid.clone();
        let result = ingest_folder(
            &ws.pool,
            ws.store.as_ref(),
            ws.embedder.as_ref(),
            &sid,
            &root,
            &ChunkConfig::default(),
            |p| {
                let _ = progress_app.emit(
                    INGEST_PROGRESS,
                    IngestProgress {
                        source_id: progress_sid.clone(),
                        path: p.path,
                        index: p.index,
                        total: p.total,
                    },
                );
            },
        )
        .await;

        let done = match result {
            Ok(summary) => {
                let _ = repo::set_source_status(&ws.pool, &sid, "ready").await;
                IngestDone {
                    source_id: sid.clone(),
                    ok: true,
                    documents: summary.documents_ingested,
                    chunks: summary.chunks_created,
                    error: None,
                }
            }
            Err(e) => {
                let _ = repo::set_source_status(&ws.pool, &sid, "error").await;
                IngestDone {
                    source_id: sid.clone(),
                    ok: false,
                    documents: 0,
                    chunks: 0,
                    error: Some(e.to_string()),
                }
            }
        };
        let _ = app.emit(INGEST_DONE, done);
    });

    Ok(source_id)
}

/// Map an ingest result + the source id into the `ingest:done` payload, setting
/// the source's terminal status as a side effect.
async fn finish_ingest(
    pool: &sqlx::SqlitePool,
    source_id: &str,
    result: AppResult<IngestSummary>,
    synced: bool,
) -> IngestDone {
    match result {
        Ok(summary) => {
            if synced {
                let _ = repo::set_source_synced(pool, source_id, now_ms()).await;
            } else {
                let _ = repo::set_source_status(pool, source_id, "ready").await;
            }
            IngestDone {
                source_id: source_id.to_string(),
                ok: true,
                documents: summary.documents_ingested,
                chunks: summary.chunks_created,
                error: None,
            }
        }
        Err(e) => {
            let _ = repo::set_source_status(pool, source_id, "error").await;
            IngestDone {
                source_id: source_id.to_string(),
                ok: false,
                documents: 0,
                chunks: 0,
                error: Some(e.to_string()),
            }
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Per-workspace clone cache root for repo sources.
fn repo_cache_root(workspace_id: &str) -> PathBuf {
    std::env::temp_dir().join("mnemos-cache").join(workspace_id)
}

fn progress_emitter(app: AppHandle, source_id: String) -> impl FnMut(DocProgress) {
    move |p: DocProgress| {
        let _ = app.emit(
            INGEST_PROGRESS,
            IngestProgress {
                source_id: source_id.clone(),
                path: p.path,
                index: p.index,
                total: p.total,
            },
        );
    }
}

/// Clone a public Git URL and ingest its working tree. Returns the source id
/// immediately; clone + ingest run in the background (PLAN §5/§9).
#[tauri::command]
pub async fn add_repo_source(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> AppResult<String> {
    let ws = state.require_active().await?;
    let source_id = uuid::Uuid::now_v7().to_string();
    repo::insert_source(&ws.pool, &source_id, "repo", &url, "ingesting").await?;

    let cache_root = repo_cache_root(&ws.id);
    let sid = source_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = ingest_repo(
            &ws.pool,
            ws.store.as_ref(),
            ws.embedder.as_ref(),
            &sid,
            &url,
            &cache_root,
            &ChunkConfig::default(),
            progress_emitter(app.clone(), sid.clone()),
        )
        .await;
        let done = finish_ingest(&ws.pool, &sid, result, false).await;
        let _ = app.emit(INGEST_DONE, done);
    });
    Ok(source_id)
}

/// Fetch a web page, extract readable text, and ingest it as one document.
#[tauri::command]
pub async fn add_url_source(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> AppResult<String> {
    let ws = state.require_active().await?;
    let source_id = uuid::Uuid::now_v7().to_string();
    repo::insert_source(&ws.pool, &source_id, "url", &url, "ingesting").await?;

    let sid = source_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = ingest_url(
            &ws.pool,
            ws.store.as_ref(),
            ws.embedder.as_ref(),
            &sid,
            &url,
            &ChunkConfig::default(),
        )
        .await;
        let done = finish_ingest(&ws.pool, &sid, result, false).await;
        let _ = app.emit(INGEST_DONE, done);
    });
    Ok(source_id)
}

/// Re-sync an existing repo/url source: fetch the latest and re-ingest changed
/// documents (idempotent on content hash). Folder sources are watched, not
/// synced, so this rejects them.
#[tauri::command]
pub async fn sync_source(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let ws = state.require_active().await?;
    let (kind, uri) = repo::source_kind_and_uri(&ws.pool, &id)
        .await?
        .ok_or_else(|| crate::error::AppError::InvalidArgument(format!("unknown source: {id}")))?;
    repo::set_source_status(&ws.pool, &id, "ingesting").await?;

    let cache_root = repo_cache_root(&ws.id);
    tauri::async_runtime::spawn(async move {
        let cfg = ChunkConfig::default();
        let result = match kind.as_str() {
            "repo" => {
                ingest_repo(
                    &ws.pool,
                    ws.store.as_ref(),
                    ws.embedder.as_ref(),
                    &id,
                    &uri,
                    &cache_root,
                    &cfg,
                    progress_emitter(app.clone(), id.clone()),
                )
                .await
            }
            "url" => {
                ingest_url(
                    &ws.pool,
                    ws.store.as_ref(),
                    ws.embedder.as_ref(),
                    &id,
                    &uri,
                    &cfg,
                )
                .await
            }
            other => Err(crate::error::AppError::InvalidArgument(format!(
                "source kind '{other}' is not syncable"
            ))),
        };
        let done = finish_ingest(&ws.pool, &id, result, true).await;
        let _ = app.emit(INGEST_DONE, done);
    });
    Ok(())
}
