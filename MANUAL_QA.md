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
