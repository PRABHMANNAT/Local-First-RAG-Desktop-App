//! Managed application state. Holds the active workspace's database pool,
//! vector store, and embedder behind a mutex so Tauri commands can share them.
//!
//! M1 supports a single active workspace; multi-workspace switching (driven by
//! the workspace rail) builds on this in a later milestone.

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::embed::Embedder;
use crate::index::VectorStore;

/// The currently open workspace and its wired-up services.
#[derive(Clone)]
pub struct ActiveWorkspace {
    pub id: String,
    pub pool: SqlitePool,
    pub store: Arc<dyn VectorStore>,
    pub embedder: Arc<dyn Embedder>,
}

/// Shared, mutable app state registered with Tauri via `manage`.
#[derive(Default)]
pub struct AppState {
    pub active: Mutex<Option<ActiveWorkspace>>,
}

impl AppState {
    /// Clone out the active workspace, or return an error if none is open.
    pub async fn require_active(&self) -> Result<ActiveWorkspace, crate::error::AppError> {
        self.active
            .lock()
            .await
            .clone()
            .ok_or_else(|| crate::error::AppError::Other("no active workspace".into()))
    }
}
