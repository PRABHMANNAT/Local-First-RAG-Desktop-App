# Progress

A running log of milestone completion. One paragraph per milestone, appended as
each is finished (newest at the bottom). See [ARCHITECTURE.md](ARCHITECTURE.md)
for the design overview.

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

## M2 — Hybrid retrieval + citation drawer

Retrieval went hybrid and citations became navigable. On the Rust side, a
lexical arm joins the vector arm: a dependency-free **Okapi BM25** index
(`index/bm25.rs`) with a shared **analyzer** (`index/text.rs`, lowercase →
alphanumeric split → small stop-word set) is built over the chunk corpus and
queried alongside vector search. The two ranked lists fuse via **Reciprocal Rank
Fusion** (`retrieve/fusion.rs`, k=60) — fusing on rank, not on incomparable
cosine/BM25 scores. **MinHash** over 3-token shingles (`retrieve/dedup.rs`, 64
hash slots) drops near-duplicate chunks while keeping the highest-ranked
representative, and a greedy **token-budget packer** (`retrieve/packing.rs`)
trims the fused set to the 6000-token context budget, admitting a smaller tail
chunk when a larger one overflows rather than truncating. `retrieve_hybrid` wires
vector ∪ BM25 → RRF → dedup → pack behind the existing `RetrievedChunk` surface,
so the answerer is untouched. As with M1's `SqliteVectorStore` standing in for
LanceDB, the in-process BM25 is a deliberate stand-in for `tantivy` — correct,
fully unit-tested, and free of the native-binding friction tantivy brings on
Windows; it can be swapped behind the same surface later. On the **frontend**,
citation markers and footnotes are now clickable and open a **source viewer
drawer** (`components/viewer/`) that switches on locator kind: `TextView`
(charspan highlight), `CodeView` (line-range highlight with gutter), and
`PdfView` (page text + char-span highlight with an optional bbox overlay,
preferring bbox and falling back to span per the citation design); `time` locators render a
timestamped transcript window. Multiple open citations become **tabs** (deduped
by chunk id, with previous-tab focus on close), backed by a `viewer` Zustand
store. Pure highlight math lives in `lib/highlight.ts` so it unit-tests in
isolation. 83 Rust tests and 22 vitest tests pass; fmt, clippy (`-D warnings`),
eslint, typecheck, and build are all green.

## M3 — Repo + URL sources

Two new source kinds joined the folder source, both following the same
ingest contract. The **repo source** (`ingest/sources/repo.rs`) parses and
validates a Git URL (https / git@ SSH forms), derives a filesystem-safe
`org__repo` cache directory, and shallow-clones (`--depth 1`) — or fetches, on a
re-sync — via the `git` binary, then reuses the folder walk to ingest the
working tree. The **URL source** (`ingest/sources/url.rs`) fetches a page with
`reqwest` and runs a small, dependency-free readability pass (drop
`<head>`/`<script>`/`<style>`/comments, strip tags, collapse whitespace, pull
the `<title>`); the pure extractor is fully unit-tested without a network.
**`.mnemosignore`** (`ingest/sources/ignore_rules.rs`) augments `.gitignore`
with identical semantics by registering it as a custom ignore filename on the
walker, so user excludes compose with a repo's own ignore rules. The pipeline
gained `ingest_url` and `ingest_repo`, sharing a new `ingest_text_document`
helper for the single-document path (idempotent on content hash), and the
command layer exposes `add_repo_source`, `add_url_source`, and `sync_source`,
each running in the background and emitting the existing
`ingest:progress`/`ingest:done` events; `sync_source` re-fetches a repo/url
source and stamps `last_synced_at`. On the **frontend**, the Add-source control
became a menu (folder / Git repository / web page), the source tree shows
per-kind icons and a Sync affordance for syncable kinds, and the sources store
tracks a `syncing` state. As deliberate stand-ins (consistent with M1/M2), the
git clone shells out to the system `git` rather than a bundled static binary,
and the clone cache lives under a temp dir pending per-workspace cache wiring.
98 Rust tests and 26 vitest tests pass; fmt, clippy (`-D warnings`), eslint,
typecheck, and build are all green.
