import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirror of the Rust `Tick` event payload. */
export interface Tick {
  seq: number;
  at_ms: number;
}

/** Event channel names, kept in sync with `src-tauri/src/events.rs`. */
export const EVENTS = {
  tick: "tick",
} as const;

/**
 * Subscribe to server-emitted `tick` events. Returns an unlisten function;
 * callers must invoke it on cleanup (e.g. in a `useEffect` teardown).
 */
export function onTick(handler: (tick: Tick) => void): Promise<UnlistenFn> {
  return listen<Tick>(EVENTS.tick, (event) => handler(event.payload));
}
