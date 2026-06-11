//! Mnemos core library. `main.rs` is a thin shim that calls [`run`]; keeping the
//! app body in a lib lets the same core drive desktop and (later) mobile.

pub mod answer;
mod commands;
pub mod db;
pub mod embed;
mod error;
mod events;
pub mod index;
pub mod ingest;
pub mod model;
pub mod retrieve;
pub mod state;

pub use error::{AppError, AppResult};

/// Initialize structured logging. Honors `RUST_LOG`; defaults to `info`.
/// A redaction layer for secrets is added when the keychain lands (M1+).
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,mnemos_lib=debug"));
    let _ = fmt().with_env_filter(filter).try_init();
}

/// Build and run the Tauri application. Registers the IPC command surface and
/// blocks until the window closes.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::app_version,
            commands::workspace::open_default_workspace,
            commands::ingest::add_folder_source,
            commands::chat::search,
            commands::chat::ask,
            events::start_tick,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mnemos");
}
