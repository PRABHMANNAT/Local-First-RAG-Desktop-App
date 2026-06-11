import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirror of the Rust `Tick` event payload. */
export interface Tick {
  seq: number;
  at_ms: number;
}

/** Event channel names, kept in sync with the Rust event/command modules. */
export const EVENTS = {
  tick: "tick",
  ingestProgress: "ingest:progress",
  ingestDone: "ingest:done",
} as const;

/** Per-file ingest progress (mirrors the Rust `IngestProgress`). */
export interface IngestProgress {
  sourceId: string;
  path: string;
  index: number;
  total: number;
}

/** Ingest completion (mirrors the Rust `IngestDone`). */
export interface IngestDone {
  sourceId: string;
  ok: boolean;
  documents: number;
  chunks: number;
  error?: string;
}

export function onIngestProgress(
  handler: (p: IngestProgress) => void,
): Promise<UnlistenFn> {
  return listen<IngestProgress>(EVENTS.ingestProgress, (e) => handler(e.payload));
}

export function onIngestDone(
  handler: (d: IngestDone) => void,
): Promise<UnlistenFn> {
  return listen<IngestDone>(EVENTS.ingestDone, (e) => handler(e.payload));
}

/**
 * Subscribe to server-emitted `tick` events. Returns an unlisten function;
 * callers must invoke it on cleanup (e.g. in a `useEffect` teardown).
 */
export function onTick(handler: (tick: Tick) => void): Promise<UnlistenFn> {
  return listen<Tick>(EVENTS.tick, (event) => handler(event.payload));
}
