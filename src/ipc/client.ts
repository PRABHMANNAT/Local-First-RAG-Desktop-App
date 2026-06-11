import { invoke } from "@tauri-apps/api/core";

/** Mirror of the Rust `Pong` struct returned by the `ping` command. */
export interface Pong {
  message: string;
  echoed: string;
}

/**
 * Typed wrapper over the Tauri command surface. The rest of the app calls these
 * methods, never `invoke` directly — so the command names and argument shapes
 * live in exactly one place and stay in sync with `src-tauri/src/commands`.
 */
export const api = {
  /** Round-trip IPC smoke test; echoes `name` back. */
  ping: (name: string): Promise<Pong> => invoke<Pong>("ping", { name }),

  /** App version baked in at compile time. */
  appVersion: (): Promise<string> => invoke<string>("app_version"),

  /** Start a bounded server-side ticker; ticks arrive on the `tick` event. */
  startTick: (count: number, intervalMs: number): Promise<void> =>
    invoke<void>("start_tick", { count, intervalMs }),
};

export type Api = typeof api;
