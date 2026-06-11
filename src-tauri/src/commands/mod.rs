//! IPC command surface. The frontend talks to the Rust core *only* through the
//! `#[tauri::command]` functions registered here — no direct DB/index access.
//!
//! M0 ships two commands that prove the request/response half of the IPC
//! contract: `ping` (echo round-trip) and `app_version`.

pub mod chat;
pub mod ingest;
pub mod workspace;

use serde::Serialize;

use crate::error::{AppError, AppResult};

/// Response for [`ping`]. Carries a fixed greeting plus the echoed input so a
/// caller can assert the full round-trip, not just that *something* came back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pong {
    pub message: String,
    pub echoed: String,
}

/// Round-trip IPC smoke test. Echoes `name` back wrapped in a [`Pong`].
/// Empty input is rejected so the error path is exercised too.
#[tauri::command]
pub fn ping(name: String) -> AppResult<Pong> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument("name must not be empty".into()));
    }
    Ok(Pong {
        message: "pong".to_string(),
        echoed: trimmed.to_string(),
    })
}

/// Returns the app version baked in at compile time. Used by the About panel
/// and to verify the command pipeline end to end on first boot.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_echoes_trimmed_input() {
        let pong = ping("  Mnemos  ".to_string()).expect("ping should succeed");
        assert_eq!(pong.message, "pong");
        assert_eq!(pong.echoed, "Mnemos");
    }

    #[test]
    fn ping_rejects_empty_input() {
        let err = ping("   ".to_string()).unwrap_err();
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn app_version_is_non_empty() {
        assert!(!app_version().is_empty());
    }
}
