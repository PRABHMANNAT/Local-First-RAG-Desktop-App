# Manual QA checklist

Run before tagging each milestone. Automated gates (CI: lint, typecheck,
vitest, clippy, cargo test) cover correctness of units; this checklist covers
what only a human at the running app can confirm. Check items per milestone and
note the date/OS.

## How to run the app

```bash
pnpm install
pnpm tauri dev      # builds the Rust core, starts Vite, opens the window
```

---

## M0 — Scaffold

Automated (verified in CI + locally):

- [x] `pnpm typecheck` clean (strict + noUncheckedIndexedAccess)
- [x] `pnpm test` green (vitest: ipc client, EmptyState, app shell)
- [x] `pnpm lint` clean
- [x] `pnpm build` produces `dist/`
- [x] `cargo fmt --all -- --check` clean
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `cargo test --all` green (ping echo/reject, tick shape, migrations, FK cascade)

Manual (verify in a desktop session):

- [ ] App boots to the empty-state shell (rail · sources · chat) under 2s
- [ ] Status bar shows `🟢 Local-only`, the app version, and `IPC: ok (N ticks)`
      (proves the `ping` round-trip and the `tick` event stream)
- [ ] Sources panel shows the "No sources yet" empty state with its action
- [ ] Chat surface shows the "Ask your sources" empty state
- [ ] Window resizes cleanly down to the 760×480 minimum
- [ ] Repeat on macOS, Windows, Linux

---

## M1 — Folder ingest + chat

Automated (verified in CI + locally):

- [x] `cargo test --all` green (52 tests: chunker, walker, embed, index, repo,
      pipeline incl. idempotent re-ingest, end-to-end retrieve, answer packing +
      citation parsing)
- [x] `pnpm test` green (10 tests incl. citation rendering, sources store)
- [x] fmt + clippy `-D warnings` clean; `pnpm build` + `pnpm typecheck` clean

Manual (desktop session; recommended with Ollama running
`ollama pull nomic-embed-text` and an instruct model):

- [ ] Click "Add folder source", pick a folder of markdown/text/code
- [ ] Sources panel shows the source ingesting with a live percentage, then
      flips to "ready" with doc/chunk counts
- [ ] Ask a question answerable from the folder → get an answer with footnote
      citations whose sources list the right files
- [ ] Ask something unrelated → the answer declines (reject-to-answer), no
      fabricated citations
- [ ] Re-add the same folder → documents are skipped (idempotent), not duplicated
- [ ] Status bar reflects 🟢 local-only throughout (no network without Ollama)
