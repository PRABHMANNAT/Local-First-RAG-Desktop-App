//! Event-stream half of the IPC contract. Commands return once; events let the
//! Rust core push a stream of updates to the webview (ingest progress, answer
//! tokens, …). M0 ships `start_tick`, a bounded ticker that proves the channel.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Name of the event channel the frontend subscribes to for ticks.
pub const TICK_EVENT: &str = "tick";

/// A single tick payload. `seq` is monotonically increasing from 0.
#[derive(Debug, Clone, Serialize)]
pub struct Tick {
    pub seq: u64,
    pub at_ms: u128,
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Emit `count` tick events spaced `interval_ms` apart on a background task.
/// Returns immediately; ticks arrive asynchronously on the [`TICK_EVENT`]
/// channel. `interval_ms` is clamped to a sane floor to avoid event floods.
#[tauri::command]
pub fn start_tick(app: AppHandle, count: u64, interval_ms: u64) {
    let interval = std::time::Duration::from_millis(interval_ms.max(16));
    tauri::async_runtime::spawn(async move {
        for seq in 0..count {
            let _ = app.emit(
                TICK_EVENT,
                Tick {
                    seq,
                    at_ms: now_ms(),
                },
            );
            tokio::time::sleep(interval).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_serializes_to_expected_shape() {
        let json = serde_json::to_value(Tick { seq: 3, at_ms: 42 }).unwrap();
        assert_eq!(json["seq"], 3);
        assert_eq!(json["at_ms"], 42);
    }
}
