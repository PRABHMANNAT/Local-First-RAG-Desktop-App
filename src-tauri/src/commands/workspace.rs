//! Workspace commands: open (creating on first run) the active workspace and
//! report which embedder is in use.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::answer::llm::{ollama::OllamaLlm, Llm, MockLlm};
use crate::db;
use crate::embed::{ollama::OllamaEmbedder, Embedder, MockEmbedder};
use crate::error::{AppError, AppResult};
use crate::index::sqlite_store::SqliteVectorStore;
use crate::state::{ActiveWorkspace, AppState};

/// Embedder dimensionality used across the workspace. Both the Ollama default
/// (`nomic-embed-text`) and the offline mock are sized to this so stored vectors
/// stay consistent within a session.
const EMBED_DIM: usize = 768;

/// Summary of the opened workspace, returned to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub embedder_model: String,
    pub embedder_dim: usize,
    /// True if a local Ollama daemon backs embeddings; false means the offline
    /// fallback is in use.
    pub ollama_detected: bool,
}

/// Open the default workspace, creating its database on first run, and register
/// it as the active workspace. Picks Ollama embeddings if a daemon is detected
/// on localhost, otherwise the offline fallback.
#[tauri::command]
pub async fn open_default_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<WorkspaceInfo> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(format!("app data dir: {e}")))?
        .join("workspaces")
        .join("default");
    std::fs::create_dir_all(&base)?;

    let pool = db::open_workspace_db(&base.join("workspace.db")).await?;

    let ollama_detected = OllamaEmbedder::detect(crate::embed::ollama::DEFAULT_BASE_URL).await;
    let embedder: Arc<dyn Embedder> = if ollama_detected {
        Arc::new(OllamaEmbedder::default_local())
    } else {
        tracing::warn!("ollama not detected; using offline mock embedder");
        Arc::new(MockEmbedder::new(EMBED_DIM))
    };

    let info = WorkspaceInfo {
        id: "default".to_string(),
        embedder_model: embedder.model_id().to_string(),
        embedder_dim: embedder.dim(),
        ollama_detected,
    };

    let llm: Arc<dyn Llm> = if ollama_detected {
        Arc::new(OllamaLlm::default_local())
    } else {
        Arc::new(MockLlm::new(None))
    };

    let store = Arc::new(SqliteVectorStore::new(pool.clone()));
    *state.active.lock().await = Some(ActiveWorkspace {
        id: "default".to_string(),
        pool,
        store,
        embedder,
        llm,
    });

    Ok(info)
}
