# Mnemos — Implementation Plan

> Local-first, privacy-focused RAG desktop app. Drop a folder, paste a Git URL, paste a YouTube URL/channel, or drag PDFs — get a chat interface with strict citations back to source. Nothing leaves the device unless you explicitly enable a remote LLM.

This document is the contract. No implementation code is written until this plan is approved. Read top to bottom; every later section assumes the vocabulary defined earlier.

---

## 1. Goals & non-goals

**Goals**
- Ingest heterogeneous sources (folder, repo, YouTube, PDF, URL) into one searchable, citable corpus per workspace.
- Answer questions with **inline, clickable citations** that resolve to the exact locator (PDF page + box, code line range, transcript timestamp).
- Be **local-first**: zero outbound traffic by default beyond explicit user actions.
- Feel calm and fast — keyboard-first, low-density, sub-2s cold start.

**Non-goals (v1)**
- No multi-user sync / cloud accounts.
- No remote reranker, no remote embeddings by default.
- No mobile. Desktop only (macOS, Windows, Linux).
- No agentic tool-use; this is retrieval + answer, not an agent framework.

---

## 2. Architecture diagram (ASCII)

```
┌──────────────────────────────────────────────────────────────────────────┐
│                              TAURI SHELL (Rust)                            │
│                                                                            │
│   ┌─────────────────────────── WEBVIEW (React 18 + TS) ───────────────┐   │
│   │                                                                    │   │
│   │  Workspace   Sources      Chat surface        Source viewer        │   │
│   │  switcher    panel        (citations)         drawer               │   │
│   │     │           │              │                   │               │   │
│   │     └───────────┴──────┬───────┴───────────────────┘               │   │
│   │                        │  Zustand stores                           │   │
│   │                        │  (workspace / sources / chat / settings)  │   │
│   │              ┌─────────┴──────────┐                                │   │
│   │              │  IPC client (typed) │   xenova fallback Web Worker   │   │
│   │              └─────────┬──────────┘   (embeddings if no Ollama)     │   │
│   └────────────────────────┼───────────────────────────────────────────┘  │
│                            │ Tauri commands (req/resp) + events (stream)   │
│   ┌────────────────────────┴───────────────────────────────────────────┐  │
│   │                      RUST CORE (src-tauri)                           │  │
│   │                                                                      │  │
│   │   commands/        events/         settings/      keychain/         │  │
│   │      │                │               │              │              │  │
│   │   ┌──┴───────────────────────────────────────────────────────────┐ │  │
│   │   │                    INGEST QUEUE (worker pool = cores-1)        │ │  │
│   │   │   sources::{folder, repo, youtube, pdf, url}                   │ │  │
│   │   │        │ produces (document, [chunk]) stream                   │ │  │
│   │   │   chunker (structural pass → sliding window)                   │ │  │
│   │   │        │                                                       │ │  │
│   │   │   embedder (Ollama nomic-embed-text │ xenova MiniLM fallback)  │ │  │
│   │   └──────────────┬─────────────────────────┬──────────────────────┘ │  │
│   │                  │                          │                        │  │
│   │          ┌───────┴────────┐        ┌────────┴─────────┐             │  │
│   │          │  SQLite (sqlx) │        │  LanceDB (vec)   │             │  │
│   │          │  metadata +    │        │  + tantivy (BM25)│             │  │
│   │          │  chunks text   │        │                  │             │  │
│   │          └───────┬────────┘        └────────┬─────────┘             │  │
│   │                  └────────────┬─────────────┘                       │  │
│   │                     RETRIEVAL (hybrid: vec ∪ BM25 → RRF → pack)     │  │
│   │                               │                                     │  │
│   │                     ANSWER (LLM: Ollama │ remote w/ user key)       │  │
│   └──────────────────────────────┼──────────────────────────────────────┘ │
│                                   │ sidecars                              │
│        yt-dlp ── whisper.cpp ── git (statically linked)                   │
└──────────────────────────────────────────────────────────────────────────┘

Network egress (only on explicit user action):
  • git clone (repo source)     • URL fetch (url source)      • yt-dlp (youtube source)
  • remote LLM call (if user configured a key)    • telemetry (only if toggled ON)
Everything else — embeddings, vector search, BM25, answering via Ollama — is in-process & local.
```

---

## 3. Data model

Authoritative metadata lives in **SQLite** (via `sqlx` migrations in `src-tauri/migrations/`). Vectors live in **LanceDB** tables derived from this schema. Full-text (BM25) lives in a **tantivy** index. SQLite is the source of truth; LanceDB and tantivy are rebuildable projections.

### 3.1 Entity-relationship (per workspace)

```
workspace 1───* source 1───* document 1───* chunk 1───1 embedding
                                              │
conversation 1───* message *───* chunk   (via citation join)
```

### 3.2 Tables

```sql
-- workspaces are physically separated: one directory + one SQLite file +
-- one LanceDB dir per workspace. This table is the *registry* (in app-config DB).
workspace(
  id TEXT PRIMARY KEY,          -- uuidv7
  name TEXT NOT NULL,
  dir  TEXT NOT NULL,           -- absolute path chosen by user
  icon TEXT,                    -- emoji or short token
  created_at INTEGER NOT NULL
)

-- everything below lives in the per-workspace SQLite db
source(
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK(kind IN ('folder','repo','youtube','pdf','url')),
  uri  TEXT NOT NULL,
  status TEXT NOT NULL,         -- queued|ingesting|ready|error|stale
  ingested_at INTEGER,
  last_synced_at INTEGER,
  meta JSON                     -- source-specific (branch, channel id, depth…)
)

document(
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES source(id) ON DELETE CASCADE,
  path_or_url TEXT NOT NULL,
  title TEXT,
  mime TEXT,
  byte_size INTEGER,
  content_hash TEXT NOT NULL,   -- blake3; drives idempotent re-ingest
  page_count INTEGER,
  ingested_at INTEGER
)

chunk(
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES document(id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL,
  text TEXT NOT NULL,
  token_count INTEGER NOT NULL,
  structural_path TEXT,         -- "H1 > H2 > section" / "fn foo()" / "00:12–00:48"
  locator JSON NOT NULL         -- discriminated union, see 3.3
)

embedding(
  chunk_id TEXT PRIMARY KEY REFERENCES chunk(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,       -- "nomic-embed-text" | "all-MiniLM-L6-v2"
  dim INTEGER NOT NULL
  -- the vector itself lives in LanceDB keyed by chunk_id; this row records
  -- provenance so a model change is detectable and re-embed is explicit.
)

conversation(
  id TEXT PRIMARY KEY, title TEXT, created_at INTEGER NOT NULL
)
message(
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversation(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK(role IN ('user','assistant','system')),
  content TEXT NOT NULL,
  created_at INTEGER NOT NULL
)
citation(
  message_id TEXT NOT NULL REFERENCES message(id) ON DELETE CASCADE,
  chunk_id   TEXT NOT NULL REFERENCES chunk(id),
  retrieved_score REAL NOT NULL,
  used_in_answer INTEGER NOT NULL,  -- bool: did the model actually cite it
  PRIMARY KEY(message_id, chunk_id)
)
```

### 3.3 Locator (discriminated union, stored as JSON in `chunk.locator`)

```ts
type Locator =
  | { kind: 'page';      page: number; char_start: number; char_end: number;
      bbox?: [number, number, number, number] }   // PDF page + char span + optional box
  | { kind: 'charspan';  char_start: number; char_end: number }  // text/markdown/url
  | { kind: 'line';      line_start: number; line_end: number }  // code files
  | { kind: 'time';      start_seconds: number; end_seconds: number } // youtube/transcript
```

The locator is what the **source viewer drawer** consumes to render the exact span. Every chunk carries exactly one locator variant; the viewer switches on `kind`.

---

## 4. Module boundaries

### 4.1 Rust core (`src-tauri/src/`)

```
lib.rs                  app bootstrap, plugin registration, managed state
commands/               #[tauri::command] surface — THE only IPC entry points
  workspace.rs          create/list/delete/export/import workspace
  source.rs             add/remove/sync source, list source tree
  ingest.rs             enqueue, cancel, status
  chat.rs               ask(), stream tokens via events, list conversations
  settings.rs           get/set settings, model lists, privacy posture
db/
  mod.rs                pool, migrations runner
  models.rs             row structs (sqlx::FromRow)
  repo.rs               typed queries (no SQL leaks above this line)
ingest/
  queue.rs              backpressured queue + worker pool (cores-1)
  pipeline.rs           (document,[chunk]) stream contract
  sources/
    folder.rs  repo.rs  youtube.rs  pdf.rs  url.rs
  chunker.rs            structural pass + sliding window
  watcher.rs            notify-based folder watch, hash-diff re-ingest
embed/
  mod.rs                Embedder trait
  ollama.rs             nomic-embed-text via localhost
  (xenova fallback lives in frontend Web Worker; bridged over IPC)
index/
  lance.rs              vector table per workspace, upsert/search
  bm25.rs               tantivy index, upsert/search
  fusion.rs             RRF merge, MinHash dedup, greedy token-budget packing
retrieve/
  mod.rs                Retriever: query → ranked chunks
  rewrite.rs            optional sub-query expansion (off by default)
answer/
  mod.rs                prompt assembly, citation marker contract
  llm/
    ollama.rs           local generate (streaming)
    remote.rs           anthropic/openai/openrouter (key from keychain)
sidecar/
  mod.rs                spawn + lifecycle for yt-dlp / whisper / git
keychain.rs             tauri-plugin-keyring wrapper; NEVER logs
events.rs               typed event payloads (ingest progress, tokens)
config.rs               settings struct, on-disk format, defaults
error.rs                AppError, thiserror; maps to IPC error shape
```

**Boundary rules**
- The frontend talks to Rust **only** through `commands/*`. No direct DB/index access from JS.
- `db/repo.rs` is the only place SQL strings exist. `commands` and `ingest` call typed methods.
- `embed`, `index`, `retrieve`, `answer` are pure-ish libraries with trait seams so they're unit-testable without a running app.

### 4.2 Frontend (`src/`)

```
main.tsx, App.tsx
ipc/                    typed wrappers over invoke()/listen() — mirrors commands/*
  client.ts             invoke<T> with error normalization
  events.ts             typed listen() subscriptions
stores/                 Zustand
  workspace.ts  sources.ts  chat.ts  settings.ts  ui.ts
components/
  layout/               WorkspaceRail, SourcesPanel, ChatSurface, ViewerDrawer
  chat/                 MessageList, CitationMarker, Composer, SlashMenu
  sources/              SourceTree, AddSourceDialog, IngestProgress, EmptyState
  viewer/               PdfView, CodeView, TextView, YoutubeView (locator switch)
  command/              CommandPalette (⌘K)
  ui/                   shadcn copy-ins (button, dialog, input, tooltip… only used ones)
workers/
  embed.worker.ts       xenova MiniLM fallback embedder
lib/
  keymap.ts             keyboard map (⌘K/N/L/1-9/\, j/k)
  tokens.ts             design tokens (colors, type scale, spacing)
  format.ts             time/byte/score formatting (rounded!)
```

---

## 5. Ingestion pipeline

A **backpressured queue** feeding a worker pool (`cores - 1`). Each source is an async producer of a `(document, [chunk])` stream into the indexer. Progress streams to the UI via Tauri events: per-source %, per-document %, and ETA (EWMA of recent doc throughput).

```
addSource ─▶ enqueue(source) ─▶ [worker pool] ─▶ resolve docs ─▶ for each doc:
                                                     hash ─▶ unchanged? skip
                                                     │ changed/new
                                                     parse ─▶ chunk ─▶ embed ─▶
                                                     upsert(SQLite, LanceDB, tantivy)
                                                     emit progress event
```

**Invariants**
- **Idempotent**: keyed on `content_hash` (blake3). Re-ingest of identical content is a no-op.
- **Resumable**: queue state persisted; on restart, `status IN ('queued','ingesting')` sources resume.
- **Fault-isolated**: a per-document failure records `document.status=error` and continues; it never kills the queue.

### Per-source contracts

| Source | Resolve | Parse | Locator | Re-ingest trigger |
|---|---|---|---|---|
| **folder** | recursive walk, respect `.gitignore` | by ext (md/txt/pdf/docx/code) | `charspan` / `line` / `page` | `notify` watch, hash-diff |
| **repo** | shallow `git clone` to workspace cache, respect `.mnemosignore` | folder logic | same as folder | user "Sync" → fetch + diff |
| **youtube** | URL/playlist/channel → video IDs (yt-dlp) | captions; else whisper.cpp | `time` | user "Sync" |
| **pdf** | direct file | pdfjs-dist per-page text + bbox | `page` (+ bbox) | hash-diff |
| **url** | `reqwest` fetch | readable extraction (dom_smoothie) | `charspan` | user "Sync"; opt-in crawl w/ depth limit |

Default include list (folder/repo) is configurable in Settings. `.mnemosignore` augments `.gitignore`.

---

## 6. Retrieval & answer pipeline

```
query
  │  (optional, off by default) rewrite → 1–3 sub-queries
  ▼
for each sub-query:
   vector top-30 (LanceDB)  ∪  BM25 top-30 (tantivy)
  ▼
RRF merge → top-12          (reciprocal rank fusion, k=60)
  ▼
(optional, off by default) cross-encoder rerank (small local model, behind flag)
  ▼
MinHash dedup near-identical chunks
  ▼
greedy pack to token budget (default 6000 tokens), highest score first
  ▼
answer prompt:  context + question + CITATION CONTRACT
  ▼
LLM (Ollama default │ remote if key configured) — stream tokens
  ▼
post-process: parse [^chunk_id] markers → superscripts; persist citation rows
```

**Reject-to-answer rule**: if the top retrieved score < threshold, the prompt instructs the model to state it lacks sufficient context and to **list what it would need** — no fabricated citations. Enforced two ways: (1) prompt instruction, (2) post-process drops any `[^chunk_id]` marker whose id wasn't in the supplied context (anti-hallucination guard).

---

## 7. Citation model

The **answer prompt** requires inline markers in the exact form `[^chunk_id]` immediately after the supported claim. Post-processing:

1. Tokenize assistant output, extract `[^<id>]` markers.
2. Validate each `id` against the context set actually sent. Unknown id → marker stripped (logged as a guard hit, not shown).
3. Map each valid marker to a footnote superscript `¹²³…` in render order; store `citation(message_id, chunk_id, retrieved_score, used_in_answer=true)`.
4. Retrieved-but-uncited chunks are stored with `used_in_answer=false` (powers a "sources consulted" affordance).

**Click behavior** — opens the **source viewer drawer** at the chunk's locator:
- `page` → render PDF page via pdfjs, draw highlight box from `bbox`/char span.
- `line` → render code file, highlight `line_start..line_end`.
- `charspan` → render text/markdown/url-extracted content, highlight span.
- `time` → embed player jumped to `start_seconds`, show the transcript window.

Multiple open citations become **tabs** in the drawer. `j/k` moves citation focus in the message; Enter opens the focused one.

---

## 8. Performance budget

| Budget | Target | How we hit it |
|---|---|---|
| Cold start → interactive | <2s mac / <4s Win | lazy-load viewers/workers; LanceDB opened on first query, not boot |
| First token | <1.5s @ 50k chunks | warm Ollama, pre-built indexes, cap retrieval at top-12 |
| Ingest 1000 md (~5KB) | <60s @ 8-core | worker pool cores-1, batch embeds, batched upserts |
| Memory ceiling | <600MB @ 100k chunks | stream parsing, no full-corpus in RAM, mmap LanceDB |
| Main-thread block | <50ms | all ingest/embed off-main (Rust threads + Web Worker) |

A budget regression is **P0 and blocks merge**. M5 adds a lightweight bench harness to guard the ingest + first-token numbers in CI (perf smoke, not a hard gate on every PR).

---

## 9. Threat model — what stays local, what doesn't

**Stays local always**: file reads, parsing, chunking, embeddings (Ollama/xenova), vector search, BM25, answer generation via Ollama, all workspace data on disk.

**Egress only on explicit user action**:
- `git clone`/fetch — when adding/syncing a **repo** source.
- HTTP fetch — when adding/syncing a **url** source (depth-limited crawl is opt-in).
- yt-dlp — when adding a **youtube** source (metadata + captions; **audio never uploaded**, whisper runs locally).
- Remote LLM call — only if the user configured a key (Anthropic/OpenAI/OpenRouter).
- Telemetry — only if the toggle is **ON** (default OFF).

**Privacy posture badge** (status bar): 🟢 Local-only · 🟡 Remote LLM enabled · 🔴 Telemetry on. The badge is computed from live settings, not cached.

**Secrets**: API keys live in the **OS keychain** via `tauri-plugin-keyring`. Never written to config files, never logged. A logging guard redacts anything matching key patterns.

**Data lifecycle**: all workspace data under one user-chosen directory. "Export workspace" → single tarball. "Delete workspace" → zeroes the directory (overwrite then remove), then drops the registry row.

---

## 10. Milestones (M0–M5) with acceptance criteria

Each milestone ends with: passing tests, a run of the manual checklist, a Conventional Commit, and a paragraph in `PROGRESS.md`.

### M0 — Scaffold
Tauri 2 + React 18 + TS strict + Vite + Tailwind v4 + needed shadcn copy-ins. IPC plumbing (one round-trip command + one event stream proven). Empty workspace + sources SQLite schema with `sqlx` migrations. Design tokens locked (Space Grotesk display, Inter body, `#FAFAF7` bg, single warm accent, hairline borders).
**Accept**: app boots to an empty-state shell on all 3 OSes; `cargo test` + `vitest` green; migrations apply cleanly; a ping command returns and an event tick renders.

### M1 — Folder ingest + chat
Folder source only. In-process embeddings (Ollama if present, else xenova worker). LanceDB wired. Naive vector-only retrieval. Basic chat with citations as **plain links** (no drawer yet).
**Accept**: drop a folder → see ingest progress → ask a question → get an answer with at least one citation link that names the source file; chunking + retrieval unit tests pass against the fixture corpus.

### M2 — Hybrid retrieval + citation drawer
tantivy BM25 + RRF fusion + MinHash dedup + token-budget packing. Source viewer drawer with PDF (page + highlight box) and text/code highlighting.
**Accept**: hybrid beats vector-only on the fixture snapshot test; clicking a citation opens the drawer at the correct locator with the right span highlighted; PDF + code + text viewers all render.

### M3 — Repo + URL sources
git sidecar (shallow clone), URL fetcher + readable extraction, `.mnemosignore` support, user-triggered Sync.
**Accept**: paste a public Git URL → clone → ingest → cite a code line range that opens to the right lines; paste a URL → ingest → cite a text span; `.mnemosignore` excludes matched paths.

### M4 — YouTube + Whisper
yt-dlp resolve (video/playlist/channel), captions path, whisper.cpp transcription fallback, timestamped citations with embedded player.
**Accept**: paste a captioned video → cite a timestamp that jumps the embedded player; paste a caption-less video → local transcription produces timestamped chunks; **no audio leaves the device** (verified by network inspection in manual QA).

### M5 — Polish
Command palette (⌘K), full keyboard map, empty/error/loading states everywhere, telemetry toggle, export/import workspace, signed builds, all docs.
**Accept**: every panel has a deliberate empty state; keyboard map fully works; export → delete → import round-trips a workspace; Playwright E2E green for the 3 core flows; signed installers produced per OS; README/ARCHITECTURE/PRIVACY/CONTRIBUTING/MANUAL_QA + demo gif present.

---

## 11. Testing strategy

- **Unit (Rust `cargo test`)**: every chunking, fusion, dedup, packing, citation-parse function. Trait seams let `embed`/`llm` be faked.
- **Unit (Vitest)**: ipc wrappers, stores, citation rendering, keymap, formatters.
- **Snapshot**: a fixture corpus (10 markdown + 2 PDFs + 1 transcript) with snapshotted retrieval results; guards ranking regressions.
- **E2E (Playwright)** against the dev build: ingest folder · ask question · click citation.
- **CI (GitHub Actions)**: matrix macOS + Windows + Linux; lint (clippy `#![deny(warnings)]`, eslint) + typecheck (`tsc`, `strict`+`noUncheckedIndexedAccess`) + test on every push.

---

## 12. Repository layout

```
/                       PLAN.md  PROGRESS.md  README.md  ARCHITECTURE.md
                        PRIVACY.md  CONTRIBUTING.md  MANUAL_QA.md  LICENSE
.github/workflows/      ci.yml  release.yml
docs/                   demo.gif  screenshots/
src/                    frontend (see 4.2)
src-tauri/              Rust core (see 4.1)
  migrations/           sqlx migrations
  fixtures/             test corpus (md, pdf, transcript)
  tauri.conf.json       sidecars, bundle, signing
tests/e2e/              Playwright specs
```

---

## 13. Commit plan (≥130 Conventional Commits)

Commits are grouped per **logical change**, not per file. Target distribution (minimum; more is fine):

| Phase | Focus | Commits |
|---|---|---|
| M0 | scaffold, tooling, CI, tokens, schema, IPC | ~18 |
| M1 | folder source, chunker, embed, lance, retrieval, chat | ~26 |
| M2 | bm25, RRF, dedup, packing, viewer drawer, highlights | ~24 |
| M3 | git sidecar, url fetch, .mnemosignore, sync | ~18 |
| M4 | yt-dlp, captions, whisper sidecar, timestamp citations | ~20 |
| M5 | palette, keymap, states, telemetry, export/import, docs, signing | ~24 |
| — | **Total** | **~130** |

Commit type vocabulary: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `build`, `ci`, `style`. Scopes mirror modules: `ingest`, `retrieve`, `answer`, `ui`, `db`, `index`, `embed`, `sidecar`, `ci`, etc. Example: `feat(retrieve): add RRF fusion over vector and bm25 candidates`.

---

## 14. Key risks & mitigations

| Risk | Mitigation |
|---|---|
| Ollama absent on user machine | xenova MiniLM Web Worker fallback; clear Settings switch |
| LanceDB / tantivy Rust binding friction on Windows | pin versions early in M0; CI Windows job catches it on first push |
| Sidecar bundling (yt-dlp/whisper/git) per-OS | resolve in M3/M4 with `tauri.conf.json` externalBin; static linking for git |
| PDF bbox highlighting accuracy | store char span + bbox; viewer prefers bbox, falls back to span |
| Perf budget regressions | M5 bench smoke; budgets documented as P0 |
| Re-embed on model change | explicit user action only; `embedding.model_id` makes drift detectable |

---

## 15. Open questions for reviewer

1. **Warm accent color** — propose a muted terracotta/amber (`#C2613D`-ish) against `#FAFAF7`. OK, or pick another?
2. **Default LLM model** — plan assumes `llama3.1:8b-instruct-q4` for the perf budget. Keep as the documented default?
3. **docx parsing** — included in the folder default list; acceptable to defer the docx parser to M1-stretch if it threatens the M1 timeline?
4. **Signing** — code signing in M5 needs certs (Apple Developer ID, Windows cert). Do you have these, or should M5 ship unsigned installers + document the signing step?

---

*Stop point: awaiting review of this plan before any implementation code is written.*
