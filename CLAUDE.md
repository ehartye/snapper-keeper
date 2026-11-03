# Project guidance for Claude sessions on snapper-keeper

**Dev environment setup (toolchain versions, per-OS prerequisites, `pnpm tauri dev`):** [`README.md`](README.md) is canonical. 

## What this repo is

Cross-platform (Windows + macOS) desktop utility — screen capture with annotation + OCR-indexed search, plus clipboard history with a caret-anchored popup. Tauri 2 (Rust + React/TS), local-first, no servers, no telemetry. Audience is "share-friendly side project" — signed installers + auto-update via GitHub Releases, no store distribution.

**Full design:** [`docs/superpowers/specs/2026-05-20-snapper-keeper-design.md`](docs/superpowers/specs/2026-05-20-snapper-keeper-design.md) — read this first if you don't have context. Section 13 has the decisions log; sections 4-10 cover architecture, data model, flows, OS integration, errors, and testing.

## Architecture rules (do not violate)

1. **One Tauri plugin per feature.** Each plugin is its own crate under `crates/`. Plugins ship paired TS bindings under `packages/`.
2. **All persistence flows through `snk-library`.** No other plugin reads or writes DB tables directly. `snk-library` exposes the typed query/mutation API.
3. **No plugin imports another plugin's internals.** Cross-plugin communication is Tauri commands or events.
4. **OCR is fire-and-forget.** Capture emits `capture:saved`; OCR plugin subscribes async. Capture never waits.
5. **Windows are frontend-only.** Plugins are pure Rust contracts. Windows live in `app/src/windows/`.
6. **`snk-clipboard` skips its own writes** so auto-copy from capture doesn't re-trigger the watcher.

Breaking any of these is a red flag — push back before implementing.

## Code conventions

- **Files >500 lines are a red flag** (Eric's standard). If you're approaching that, split the module.
- **Rust style:** workspace inherits edition 2021, Rust 1.78+. `rustfmt.toml` uses stable-only options.
- **TS style:** strict mode + `noUncheckedIndexedAccess`. ESLint v9 flat config (`eslint.config.mjs`). React rules scoped to `**/*.tsx` only (non-React TS packages would warn otherwise).
- **Errors cross the IPC boundary as typed enums** (`LibraryError`, `CaptureError`, eventually `AppError`). The discriminator tag in serde is `"kind"`, so variant field names must NOT be `kind` — use `reason`, `detail`, `code`, etc.


## Release pipeline (post-hardening)

The release pipeline is split into 7 jobs gated on a `production-release` GitHub environment:

```
tag (v*) → verify-pubkey → build (matrix, NO secrets) → artifact-verify
       → environment gate (production-release, ehartye approves)
       → sign-mac-arm / sign-mac-x64 / sign-win-x64 (each gets only its platform's secrets)
       → publish-release (gh release create + CycloneDX SBOM upload)
```

Plus a scheduled `audit.yml` (daily at 06:17 UTC; also `workflow_dispatch`) that runs `cargo-audit` + `pnpm audit` and manages per-advisory issues via `.github/scripts/sync-audit-issues.js`.

**To approve a tag-triggered release deployment:**
```bash
# Pending deployments for the run
pending="$(gh api repos/ehartye/snapper-keeper/actions/runs/<RUN_ID>/pending_deployments)"
echo "$pending"

# Approve the production-release environment for this run
env_id="$(echo "$pending" | jq -r '.[] | select(.environment.name == "production-release") | .environment.id')"
echo "{\"environment_ids\":[${env_id}],\"state\":\"approved\",\"comment\":\"...\"}" \
  | gh api -X POST repos/ehartye/snapper-keeper/actions/runs/<RUN_ID>/pending_deployments --input -
```

**Source-of-truth docs:**
- Design: `docs/superpowers/specs/2026-05-24-release-pipeline-hardening-design.md`