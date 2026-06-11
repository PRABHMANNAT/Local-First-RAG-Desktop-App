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
