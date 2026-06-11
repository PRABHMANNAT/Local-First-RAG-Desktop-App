# Progress

A running log of milestone completion. One paragraph per milestone, appended as
each is finished (newest at the bottom). See [PLAN.md](PLAN.md) for the full
roadmap and acceptance criteria.

## M0 — Scaffold

The skeleton is in place. The frontend is Vite + React 18 + TypeScript in strict
mode (`strict` + `noUncheckedIndexedAccess`), styled with Tailwind v4 and the
locked design tokens — Space Grotesk for display, Inter for body, off-white
`#FAFAF7` paper, a single terracotta accent, hairline borders, no shadows. Fonts
are self-hosted via `@fontsource` so nothing is fetched at runtime. The Tauri 2
Rust core compiles behind a thin `main.rs` shim over `mnemos_lib::run()`, exposes
the IPC contract's two halves — a `ping` request/response command and a `tick`
event stream — and the webview proves both on boot via the status bar's IPC
health badge. SQLite schema lives in `sqlx` migrations split into the app
registry (`workspace`, `app_setting`) and the per-workspace schema (`source`,
`document`, `chunk`, `embedding`, `conversation`, `message`, `citation`), with
foreign-key cascade verified by unit tests. The app shell renders all three
primary surfaces — workspace rail, collapsible sources panel, chat surface —
each with a deliberate empty state, plus a status bar showing the 🟢 local-only
privacy posture. CI runs lint, typecheck, and tests across macOS, Windows, and
Linux. Frontend checks (typecheck, vitest, eslint, build) are green.

## M1 — Folder ingest + chat

The first end-to-end RAG slice works. The Rust core now has a complete ingest →
retrieve → answer pipeline, each stage seamed behind a trait so it unit-tests
without external services. The **chunker** does a structural pass (markdown
headings, fenced code) then a sentence/line sliding window (300–500 tokens, 50
overlap), carrying a heading path and a byte-span `Locator` per chunk. The
**folder source** walks gitignore-aware (via the `ignore` crate, applied even
outside git repos) and content-hashes with blake3 for idempotent re-ingest. The
**embedder** is a trait with an Ollama `nomic-embed-text` adapter (auto-detected
on localhost) and a deterministic mock for offline/testing; the **vector store**
is likewise a trait, with an M1 brute-force cosine implementation over vectors
stored in SQLite (the LanceDB adapter drops in at M2 behind the same `VectorStore`
trait). The **pipeline** ties walk → hash → skip-if-unchanged → chunk → embed →
persist, isolating per-document failures, and reports progress via a callback the
command layer turns into `ingest:progress`/`ingest:done` events. **Retrieval**
embeds the query, vector-searches top-k, and joins chunk text + document path; an
end-to-end test ingests a 3-topic corpus and confirms the right document ranks
first. The **answer** path packs a 6000-token budgeted context, prompts for
inline `[^chunk_id]` citations, strips any citation whose id wasn't supplied
(anti-hallucination), enforces a reject-to-answer threshold, and persists the
conversation, messages, and citation rows. On the **frontend**, the sources panel
shows a live ingest progress tree fed by events, a folder picker (Tauri dialog
plugin) registers sources, and the chat surface has a composer plus messages that
render citation markers as footnote superscripts with a sources list. 52 Rust
tests and 10 vitest tests pass; fmt and clippy (`-D warnings`) are clean.

Live answers require a local Ollama daemon (`nomic-embed-text` + an instruct
model); without it the app falls back to the offline mock so the pipeline still
runs for development. Verifying answers against real Ollama is a manual-QA step.
