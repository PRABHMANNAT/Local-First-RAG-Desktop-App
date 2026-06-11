use serde::{Serialize, Serializer};

/// Application-wide error type. Every Tauri command returns `AppResult<T>` so
/// failures cross the IPC boundary as a string the frontend can surface.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

// Tauri requires command errors to be `Serialize` to send them to the webview.
// We emit the human-readable message; structured variants can be added later.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
