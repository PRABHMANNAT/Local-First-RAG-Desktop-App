import { invoke } from "@tauri-apps/api/core";

/** Mirror of the Rust `Pong` struct returned by the `ping` command. */
export interface Pong {
  message: string;
  echoed: string;
}

/** Workspace summary returned by `open_default_workspace`. */
export interface WorkspaceInfo {
  id: string;
  embedderModel: string;
  embedderDim: number;
  ollamaDetected: boolean;
}

/** A locator discriminated union, mirrored from the Rust `Locator`. */
export type Locator =
  | { kind: "page"; page: number; char_start: number; char_end: number; bbox?: [number, number, number, number] }
  | { kind: "charspan"; char_start: number; char_end: number }
  | { kind: "line"; line_start: number; line_end: number }
  | { kind: "time"; start_seconds: number; end_seconds: number };

/** A citation attached to an answer. */
export interface Citation {
  chunkId: string;
  text: string;
  structuralPath: string | null;
  locator: Locator | null;
  pathOrUrl: string;
  score: number;
  usedInAnswer: boolean;
}

/** The result of asking a question. */
export interface Answer {
  conversationId: string;
  answer: string;
  citations: Citation[];
  rejected: boolean;
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

  /** Open (creating on first run) the default workspace and make it active. */
  openDefaultWorkspace: (): Promise<WorkspaceInfo> =>
    invoke<WorkspaceInfo>("open_default_workspace"),

  /** Add a folder as a source; returns the source id. Ingest runs in the
   * background and reports via `ingest:progress` / `ingest:done` events. */
  addFolderSource: (path: string): Promise<string> =>
    invoke<string>("add_folder_source", { path }),

  /** Clone a public Git URL and ingest it as a source; returns the source id.
   * The shallow clone is the only network egress (PLAN §9). */
  addRepoSource: (url: string): Promise<string> =>
    invoke<string>("add_repo_source", { url }),

  /** Fetch a web URL, extract readable text, and ingest it; returns the id. */
  addUrlSource: (url: string): Promise<string> =>
    invoke<string>("add_url_source", { url }),

  /** Re-sync a repo/url source (fetch + re-ingest changed documents). */
  syncSource: (id: string): Promise<void> => invoke<void>("sync_source", { id }),

  /** Ask a question; returns a cited answer (or a declined one). */
  ask: (query: string): Promise<Answer> => invoke<Answer>("ask", { query }),
};

export type Api = typeof api;
