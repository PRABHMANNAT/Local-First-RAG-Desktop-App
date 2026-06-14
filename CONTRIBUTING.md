# Contributing to Mnemos

Thanks for your interest in improving Mnemos! This document covers how to get
set up, the conventions we follow, and the quality bar a change must clear
before it merges.

## Ground rules

- **Local-first is a hard constraint.** No change may introduce network egress
  that isn't an explicit, user-initiated action (see the threat model in
  [ARCHITECTURE.md](ARCHITECTURE.md)). New outbound calls require discussion first.
- **The architecture is the contract.** [ARCHITECTURE.md](ARCHITECTURE.md)
  defines the design, data model, and module boundaries. Significant deviations
  get documented in [PROGRESS.md](PROGRESS.md) with the rationale.
- **Trait seams everywhere.** `embed`, `index`, `retrieve`, and `answer` are
  unit-testable without a running app or external services. Keep them that way.

## Getting set up

See [README.md](README.md#getting-started) for prerequisites and the run/build
commands. In short: `pnpm install`, then `pnpm tauri dev`.

## Development workflow

1. **Branch** off `main` for your change.
2. **Make focused commits** — one logical change per commit, not one per file.
3. **Run the quality gates locally** (see below) before opening a PR.
4. **Open a PR** describing what changed and why; link any related issue.

## Commit conventions

We use [Conventional Commits](https://www.conventionalcommits.org/). The
subject line is `type(scope): summary`:

- **types:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`,
  `build`, `ci`, `style`
- **scopes** mirror the modules: `ingest`, `retrieve`, `answer`, `ui`, `db`,
  `index`, `embed`, `sidecar`, `ci`, …

Examples:

```
feat(retrieve): add RRF fusion over vector and bm25 candidates
fix(ingest): isolate per-document failures in the folder walk
docs: record M2 progress in PROGRESS.md
```

## Quality gates

Every change must keep all of these green. CI enforces them across macOS,
Windows, and Linux, but please run them locally first.

### Frontend

```bash
pnpm typecheck     # tsc, strict
pnpm lint          # eslint
pnpm test          # vitest
pnpm build         # production build
```

### Rust core (from `src-tauri/`)

```bash
cargo test --all
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Clippy runs with `-D warnings`: warnings are errors. Prefer fixing the root
cause over `#[allow(...)]`; when an allow is genuinely warranted (e.g. a
builder with many arguments), scope it as narrowly as possible.

## Testing expectations

- **Pure logic** (chunking, fusion, dedup, packing, citation parsing, locator
  highlighting) gets direct unit tests.
- **New source kinds** add an ingest test against a small fixture, exercising
  the idempotent re-ingest path.
- **Frontend stores and helpers** get vitest coverage; components get a render
  test where behavior is non-trivial.

## Code style

- Match the surrounding code's naming, comment density, and idioms.
- Comments explain *why*, not *what*. Keep them honest and current.
- No secrets in code, config, or logs — ever. API keys belong in the OS
  keychain.

## Reporting bugs & proposing features

Open an issue with clear reproduction steps (for bugs) or a concrete use case
and proposed approach (for features). For anything that touches the privacy
posture or the data model, start with an issue before writing code.

By contributing, you agree that your contributions are licensed under the
project's [MIT License](LICENSE).
