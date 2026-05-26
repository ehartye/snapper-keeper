# Snapper Keeper Agent Instructions

Use this repo’s existing docs as the source of truth. Keep instructions here short, actionable, and specific to behavior that is easy to miss.

## First Reads

- Read [README.md](README.md) for setup, commands, and architecture overview.
- Read [CLAUDE.md](CLAUDE.md) for project conventions, gotchas, and phase context.
- Read [docs/superpowers/specs/2026-05-20-snapper-keeper-design.md](docs/superpowers/specs/2026-05-20-snapper-keeper-design.md) for the full design and decisions log when you need deeper context.

## Build And Test

- Dev: `pnpm --filter @snk/app tauri dev`
- Release build: `pnpm --filter @snk/app tauri build`
- TypeScript: `pnpm lint` and `pnpm typecheck`
- Rust: `cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater`
- Format and lint: `cargo fmt -- --check` and `cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings`

## Architecture Constraints

- Keep one Tauri plugin per feature, with Rust crates in `crates/` and matching TypeScript bindings in `packages/`.
- Route all persistence through `snk-library`; no other plugin should read or write database tables directly.
- Do not import another plugin’s internals; use Tauri commands or events for cross-plugin communication.
- Treat OCR as fire-and-forget: capture emits `capture:saved`, and OCR subscribes asynchronously.
- Keep windows frontend-only; Rust plugins stay pure and windows live in `app/src/windows/`.
- Make `snk-clipboard` ignore its own writes so auto-copy does not re-trigger the watcher.

## Gotchas

- On Windows, run `tauri dev` from an interactive desktop session, not SSH.
- Keep `icons/icon.ico` in place for Windows builds.
- Inline `tauri = { version = "2", features = [...] }` in `app/src-tauri/Cargo.toml`; do not rely on workspace inheritance for Tauri features.
- When adding IPC errors, keep serde’s discriminator tag as `kind`, but do not name a variant field `kind`; use `reason`, `detail`, or `code` instead.
- Treat files over 500 lines as a split-point warning.

## Docs To Link

- [docs/release-signing.md](docs/release-signing.md) for signing and release setup.
- [docs/superpowers/plans/](docs/superpowers/plans/) for phase plans and task breakdowns.