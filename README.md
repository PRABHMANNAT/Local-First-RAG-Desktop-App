<div align="center">

# Mnemos

**A local-first, privacy-focused RAG desktop app with strict citations.**

Drop a folder, paste a Git URL, paste a web link, or drag in PDFs — then chat
with your corpus and get answers whose every claim links back to the exact
source span. Nothing leaves your device unless you explicitly ask it to.

`Tauri 2` · `Rust` · `React 18 + TypeScript` · `SQLite` · `Ollama / offline fallback`

</div>

---

## Why Mnemos

Most "chat with your documents" tools send your files to a cloud service and
hand back answers you have to take on faith. Mnemos inverts both of those:

- **Local-first by default.** Embeddings, vector search, full-text search, and
  answer generation all run in-process on your machine. The only time bytes
  leave the device is an explicit action you take — cloning a repo, fetching a
  URL, or enabling a remote LLM with your own key.
- **Strict, clickable citations.** Every answer cites its sources with inline
  markers that resolve to the precise locator: a PDF page + box, a code line
  range, a text character span, or a transcript timestamp. Claims the corpus
  can't support are declined, not fabricated.
- **Calm and fast.** Keyboard-first, low-density UI; sub-2s cold start as a
  budget, not an aspiration.

## Features

| Area | What works today |
|---|---|
| **Sources** | Folders (gitignore-aware), public Git repos (shallow clone), and web pages (readable extraction). PDFs and YouTube are on the roadmap. |
| **Ingestion** | Backpressured worker pipeline: walk → hash → skip-if-unchanged → chunk → embed → index. Idempotent on a blake3 content hash; per-document failures are isolated. |
| **Retrieval** | Hybrid **vector ∪ BM25** candidate generation, fused with **Reciprocal Rank Fusion**, **MinHash** near-duplicate removal, and greedy token-budget packing. |
| **Answering** | Local LLM via Ollama (or a deterministic offline mock), a strict citation contract, and an anti-hallucination guard that strips any citation not actually supplied as context. |
| **Citations** | A tabbed **source viewer drawer** that highlights the cited span — text, code line range, or PDF page + bounding box. |
| **Privacy** | A live status-bar posture badge: 🟢 local-only · 🟡 remote LLM · 🔴 telemetry. API keys live in the OS keychain, never in config or logs. |

## Architecture at a glance

```
┌──────────────────────── Tauri shell (Rust) ────────────────────────┐
│  React 18 + TS  ──IPC commands / events──▶  Rust core               │
│  · workspace rail                            · ingest pipeline       │
│  · sources panel                             · embed (Ollama|mock)   │
│  · chat surface + citations                  · index (vector + BM25) │
│  · source viewer drawer                      · retrieve (RRF+dedup)  │
│                                              · answer (LLM + guard)  │
│                          SQLite (source of truth) ──┘                │
└─────────────────────────────────────────────────────────────────────┘
```

- **SQLite is the source of truth**; the vector index and BM25 index are
  rebuildable projections.
- Each subsystem (`embed`, `index`, `retrieve`, `answer`) sits behind a trait
  seam, so it unit-tests without a running app or any external service.
- See [ARCHITECTURE.md](ARCHITECTURE.md) for the design, data model, and threat
  model.

## Getting started

### Prerequisites

- **Node** ≥ 20 and **pnpm** 10
- **Rust** 1.88.0 (pinned via `rust-toolchain.toml`) with the MSVC toolchain on Windows
- *(optional)* **[Ollama](https://ollama.com)** running locally with
  `nomic-embed-text` and an instruct model — without it, Mnemos falls back to a
  deterministic offline mock so the full pipeline still runs for development.

### Install & run

```bash
pnpm install            # install frontend deps
pnpm tauri dev          # run the desktop app (frontend + Rust core)
```

Frontend-only iteration (no Rust):

```bash
pnpm dev                # Vite dev server
```

### Build

```bash
pnpm tauri build        # produce a desktop installer for your OS
```

## Development

| Task | Command |
|---|---|
| Frontend typecheck | `pnpm typecheck` |
| Frontend lint | `pnpm lint` |
| Frontend tests | `pnpm test` |
| Frontend build | `pnpm build` |
| Rust tests | `cargo test --all` (from `src-tauri/`) |
| Rust format | `cargo fmt --all` |
| Rust lint | `cargo clippy --all-targets -- -D warnings` |

> **Windows note:** the Rust core needs the MSVC environment on `PATH`. If
> `cargo` can't find the linker from your shell, launch it from a "x64 Native
> Tools" prompt (or a wrapper that sources `vcvars64.bat`) and set
> `RUST_MIN_STACK=134217728` to avoid a rustc stack overflow in the `windows`
> crate.

CI runs lint, typecheck, and the full test suite across macOS, Windows, and
Linux on every push.

## Project status

Built milestone-by-milestone (see [PROGRESS.md](PROGRESS.md) and [ARCHITECTURE.md](ARCHITECTURE.md)):

- ✅ **M0 — Scaffold:** Tauri + React + TS strict, design tokens, SQLite migrations, IPC plumbing.
- ✅ **M1 — Folder ingest + chat:** chunker, embeddings, vector store, retrieval, cited answers.
- ✅ **M2 — Hybrid retrieval + citation drawer:** BM25 + RRF + MinHash dedup + packing; tabbed source viewer with highlighting.
- 🚧 **M3 — Repo + URL sources:** shallow git clone, URL readable extraction, `.mnemosignore`, user-triggered Sync.
- ⏳ **M4 — YouTube + Whisper**, **M5 — Polish** (command palette, export/import, signed builds).

## Tech stack

- **Shell:** Tauri 2
- **Frontend:** React 18, TypeScript (strict + `noUncheckedIndexedAccess`), Vite, Tailwind v4, Zustand
- **Core:** Rust, `sqlx` (SQLite), `tokio`, `blake3`, `ignore`, `reqwest`
- **Models:** Ollama (`nomic-embed-text` + instruct) with a dependency-free offline fallback

## Contributing

Contributions are welcome — please read [CONTRIBUTING.md](CONTRIBUTING.md) for
the workflow, commit conventions, and quality gates.

## License

[MIT](LICENSE) © Mnemos contributors
