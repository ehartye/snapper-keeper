# snapper-keeper

Cross-platform (Windows + macOS) screen capture and clipboard manager.

> **Status:** phase 1 (foundation + vertical slice). Not yet usable for daily work — see `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md` for the design and `docs/superpowers/plans/` for active plans.

## Development

Prereqs: Rust 1.78+, Node 20+, pnpm 9+, Tauri platform deps (https://tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm --filter @snk/app tauri dev
```

Build a release bundle:

```bash
pnpm --filter @snk/app tauri build
```

## Architecture

One Tauri plugin per feature. All persistence flows through `crates/snk-library`. See the design doc for the full plugin set and dependency rules.
