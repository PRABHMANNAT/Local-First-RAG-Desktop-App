//! Ingest commands. Adding a folder source kicks off ingestion on a background
//! task and streams progress to the UI via events.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db::repo;
use crate::error::AppResult;
use crate::ingest::chunker::ChunkConfig;
use crate::ingest::pipeline::ingest_folder;
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
