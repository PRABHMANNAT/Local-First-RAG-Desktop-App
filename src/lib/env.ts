/**
 * Runtime environment helpers. The same bundle runs inside the Tauri webview
 * (where IPC works) and in a plain browser during tests / Storybook-style dev,
 * so call sites guard on `isTauri()` before invoking native commands.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
