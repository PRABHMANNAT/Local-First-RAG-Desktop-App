# Architecture

> This is the living architecture overview. It grows with each milestone; the
> full diagram, module map, and design rationale are finalized at M5. Milestone
> status is tracked in [PROGRESS.md](PROGRESS.md).

## Shape

Mnemos is a single desktop binary: a **Tauri 2** shell with a **Rust** core and
a **React + TypeScript** webview. There is no server. All data lives on disk
under a user-chosen directory, one SQLite file and one LanceDB directory per
workspace.

```
React webview  ──IPC commands──▶  Rust core  ──▶  SQLite (truth)
   (UI/state)  ◀──IPC events────  (ingest /        LanceDB (vectors)
                                   retrieve /       tantivy (BM25)
                                   answer)          sidecars: git, yt-dlp, whisper
```

## Why these choices

- **Tauri over Electron** — system webview, ~10× smaller binaries, a Rust core
  where the heavy lifting (parsing, embedding, search) belongs, and a real
  permission/capability model for a privacy-first app.
- **LanceDB** — embedded, in-process vector store; no separate service to run,
  columnar on-disk format that memory-maps, scales to 100k+ chunks within budget.
- **Hybrid retrieval (vector ∪ BM25 → RRF)** — dense embeddings miss exact terms
  (identifiers, error codes); BM25 catches them. Reciprocal rank fusion merges
  both without a remote reranker, keeping everything local.
- **SQLite as source of truth** — the vector and full-text indexes are
  rebuildable projections keyed on `chunk.id`; metadata, conversations, and
  citations need transactional integrity.

## Boundaries

The webview talks to Rust **only** through the `#[tauri::command]` surface in
`src-tauri/src/commands`. SQL is confined to the db layer; `embed`, `index`,
`retrieve`, and `answer` are trait-seamed libraries that unit-test without a
running app.

## Module map (current)

| Area | Location | State |
|---|---|---|
| IPC commands | `src-tauri/src/commands` | `ping`, `app_version` (M0) |
| Events | `src-tauri/src/events.rs` | `tick` stream (M0) |
| DB / migrations | `src-tauri/src/db`, `src-tauri/migrations` | full schema (M0) |
| Frontend IPC | `src/ipc` | typed client + event wrappers (M0) |
| State | `src/stores` | Zustand: ui, workspace (M0) |
| Layout | `src/components/layout` | rail, sources, chat shell (M0) |

Ingestion, retrieval, and answer modules land M1–M4 per the plan.
