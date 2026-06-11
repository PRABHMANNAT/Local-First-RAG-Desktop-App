//! Chat commands. M1 ships `search` (retrieval only), which the chat surface
//! uses to show grounded results with citation links. The streaming `ask`
//! command with an LLM-composed answer lands with the answer pipeline.

use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::error::AppResult;
use crate::retrieve::retrieve;
use crate::state::AppState;

/// A retrieved chunk shaped for the frontend. `locator` is the parsed JSON
/// object (not a string) so the citation drawer can switch on `kind`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedChunkDto {
    pub chunk_id: String,
    pub text: String,
    pub structural_path: Option<String>,
    pub locator: Value,
    pub path_or_url: String,
    pub score: f32,
}

/// Retrieve the top-`k` chunks relevant to `query` from the active workspace.
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    k: usize,
) -> AppResult<Vec<RetrievedChunkDto>> {
    let ws = state.require_active().await?;
    let hits = retrieve(
        &ws.pool,
        ws.store.as_ref(),
        ws.embedder.as_ref(),
        &query,
        k.clamp(1, 50),
    )
    .await?;

    Ok(hits
        .into_iter()
        .map(|h| RetrievedChunkDto {
            locator: serde_json::from_str(&h.locator).unwrap_or(Value::Null),
            chunk_id: h.chunk_id,
            text: h.text,
            structural_path: h.structural_path,
            path_or_url: h.path_or_url,
            score: h.score,
        })
        .collect())
}
