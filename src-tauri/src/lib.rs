//! Mnemos core library. `main.rs` is a thin shim that calls [`run`]; keeping the
//! app body in a lib lets the same core drive desktop and (later) mobile.

mod commands;
pub mod db;
mod error;
mod events;

pub use error::{AppError, AppResult};

/// Initialize structured logging. Honors `RUST_LOG`; defaults to `info`.
/// A redaction layer for secrets is added when the keychain lands (M1+).
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
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
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::app_version,
            events::start_tick,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mnemos");
}
