# Local installer build (unsigned) — design

**Date:** 2026-05-26
**Status:** Implemented (PR #141). Spec amended after implementation — see Phase 10 amendment below.
**Scope:** Make `pnpm build:local` produce an installer locally on Windows and macOS that is production-fidelity in every dimension except code signing. Remove the dead-weight `bundle.windows.signCommand` from base config and simplify the CI release workflow accordingly.

> ### Phase 10 amendment (2026-05-26)
>
> Phase 10 ([PR #135](https://github.com/ehartye/snapper-keeper/pull/135)) merged into `main` after this spec was written but before the implementation finished. Phase 10 removed Tesseract entirely from the project, replacing it with Apple Vision (macOS) and Windows.Media.Ocr (Windows) — both native OS APIs requiring no bundling.
>
> Sections below that describe **Tesseract bundling on Windows as production-parity** (the Approach section, the Per-OS behavior table, the Windows pre-build flow, parts of the Validation plan) are **historical** — they reflect the design at the time of writing. The shipped implementation does NOT bundle Tesseract; `scripts/build-local.sh` has no pre-build step on either OS. Production parity holds for everything else.
>
> The plan at [`docs/superpowers/plans/2026-05-26-local-installer-build.md`](../plans/2026-05-26-local-installer-build.md) is the source of truth post-merge — see its `## Plan amendment 2026-05-26 — Phase 10 merged mid-flight` section and Tasks 10 + 11 for the cleanup details.

## Motivation

Today, a contributor or maintainer who runs `pnpm tauri build` on a fresh clone hits two failures:

1. On Windows, Tauri tries to invoke `sign code artifact-signing ...` (configured under `bundle.windows.signCommand` in `app/src-tauri/tauri.conf.json`), which requires Azure Artifact Signing credentials that no contributor will have.
2. On any OS, `createUpdaterArtifacts: true` causes the bundler to demand `TAURI_SIGNING_PRIVATE_KEY` so it can sign the updater manifest with minisign.

The CI release workflow works around both by mutating `tauri.conf.json` in place (`jq 'del(.bundle.windows.signCommand)'`) and passing `--config '{"bundle":{"createUpdaterArtifacts":false}}'` to disable updater bundling. The workarounds exist nowhere else, so contributors must reverse-engineer the YAML to produce a working local build. The README's "Build a release bundle" section still documents a `pnpm --filter @snk/app tauri build` invocation that doesn't actually work without secrets.

Goals:

- One command — `pnpm build:local` — that produces an unsigned but otherwise production-fidelity installer on the contributor's native OS / architecture.
- Serve both audiences equally: maintainer smoke-testing what end users will receive before tagging a release, and external contributors validating PR changes against a real installer.
- Bundle parity with production: same target triples, same `--bundles` selections, same Tesseract bundling on Windows. Only signing differs.
- Refactor away the dead-weight `signCommand` field in the base config, since the actual signing step in CI invokes `sign code` directly rather than through Tauri's hook.

## Non-goals

- A signed local build path. If a maintainer wants to sign locally, they can set the right env vars and invoke `pnpm tauri build` directly with their own overlay — `build:local` is explicitly the unsigned path.
- Linux installers. Production targets Windows + macOS only; the script aborts on Linux with a directive to use `pnpm tauri dev` for development.
- Cross-compilation. Each contributor builds for their machine's native arch only (matches CI, where each runner builds one triple).
- Auto-installation of Tesseract on Windows. The script fails with an actionable error if Tesseract is not already installed via the methods documented in the README.
- Producing the macOS updater payload (`.app.tar.gz` + minisign `.sig`). Requires `TAURI_SIGNING_PRIVATE_KEY`.

## Approach

Production parity for what goes into the bundle, with two narrowly scoped escapes for signing:

- `bundle.windows.signCommand` is removed from `app/src-tauri/tauri.conf.json` entirely. The base config becomes buildable without any secrets. Production Windows signing remains intact because the `sign-win-x64` CI job (`.github/workflows/release.yml`:495–524) downloads the unsigned `*-setup.exe` and invokes `sign code ...` directly — it does not depend on Tauri's `signCommand` hook.
- `createUpdaterArtifacts: false` is applied via inline `--config` overlay at build time (both locally and in the CI build job — the CI sign jobs recreate the updater payload from the signed artifacts).

A single bash script `scripts/build-local.sh` is the entry point. It runs natively on macOS and inside Git Bash on Windows (which contributors already have installed because release.yml's Windows build job depends on it). pnpm exposes it as `pnpm build:local`.

## Per-OS behavior

| OS            | Target triple              | `--bundles` | Tesseract bundling                                                   | Output                            |
|---------------|----------------------------|-------------|----------------------------------------------------------------------|-----------------------------------|
| Windows x64   | `x86_64-pc-windows-msvc`   | `nsis`      | Copy from contributor's existing install into `app/src-tauri/resources/tesseract/` | `*-setup.exe`                     |
| macOS arm64   | `aarch64-apple-darwin`     | `app,dmg`   | None (production parity — macOS installers don't ship Tesseract)     | `Snapper Keeper.app` + `.dmg`     |
| macOS x86_64  | `x86_64-apple-darwin`      | `app,dmg`   | None                                                                 | same                              |
| Linux         | n/a                        | n/a         | n/a — script exits with directive to use `pnpm tauri dev`            | n/a                               |

### macOS DMG generator

The script uses Tauri's built-in DMG bundler (`--bundles app,dmg`) rather than Homebrew's `create-dmg`. The DMG layout differs cosmetically from production (the prod release uses `create-dmg` with custom `--volname`, `--window-size`, `--icon-size` in release.yml:269–274). The installable contents are functionally identical; only window dimensions and icon positioning vary. This trades cosmetic layout parity for a lower contributor barrier (no extra brew dep). If strict layout parity becomes necessary later, swapping to `create-dmg` in the script is a one-block change.

## Script flow

`scripts/build-local.sh` is the only new file. It follows this flow:

### Preamble (all OSes)

- `set -euo pipefail`; `ERR` trap prints the failing line number; `EXIT` trap cleans up the copied Tesseract files on Windows (preserving `.placeholder` — see Windows pre-build step 4).
- Resolve repo root via `git rev-parse --show-toplevel` so the script works from any subdirectory.
- Detect OS + arch via `uname -s` / `uname -m`. Compute target triple + bundles per the table above.
- Linux → exit 1 with `"Local installer build is supported on Windows + macOS only (matches production targets). Run 'pnpm tauri dev' to develop on Linux."`

### Windows pre-build

1. Resolve a Tesseract source directory in this order, mirroring `snk-ocr/sidecar.rs`'s runtime resolver:
   - `$SNK_TESSERACT_PATH` (directory containing `tesseract.exe`)
   - `where.exe tesseract`
   - `C:\Program Files\Tesseract-OCR\tesseract.exe`
2. If not found → exit 1 with `"Tesseract not found. Install via 'winget install UB-Mannheim.TesseractOCR' or 'choco install tesseract' (see README → Prerequisites). Set SNK_TESSERACT_PATH to override."`
3. `cp -r <tesseract-dir>/* app/src-tauri/resources/tesseract/`. This path is the one already declared as a bundle resource in `tauri.conf.json:120` and already gitignored at `.gitignore:23-24` (a checked-in `.placeholder` file keeps the directory present so the bundler's resource glob matches in dev).
4. The `EXIT` trap deletes everything in `app/src-tauri/resources/tesseract/` **except** `.placeholder`, on success or failure. Preserving `.placeholder` is mandatory — if it is removed, subsequent `pnpm tauri dev` / `pnpm tauri build` runs fail because the resource glob `resources/tesseract/**/*` matches nothing. Implementation: `find app/src-tauri/resources/tesseract -mindepth 1 ! -name '.placeholder' -delete` (or equivalent).

### macOS pre-build

Nothing.

### Build invocation (both OSes)

```bash
pnpm --filter @snk/app tauri build \
  --target "$TARGET" \
  --bundles "$BUNDLES" \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

The inline `--config` overlay disables updater-artifact generation, which would otherwise demand `TAURI_SIGNING_PRIVATE_KEY`.

### Post-build (both OSes)

1. Locate artifacts via globbing `target/$TARGET/release/bundle/`:
   - Windows: `nsis/*-setup.exe`
   - macOS: `macos/Snapper Keeper.app` and `dmg/Snapper Keeper_*.dmg`
2. Print a summary table per artifact:
   ```
   Built unsigned installer:
     Path:   target/x86_64-pc-windows-msvc/release/bundle/nsis/Snapper Keeper_0.1.2_x64-setup.exe
     Size:   142.7 MB
     SHA256: 1a2b3c...
   ```
3. Print install instructions for an unsigned build:
   - Windows: "SmartScreen will warn — click 'More info' → 'Run anyway'."
   - macOS: "Right-click the `.app` → 'Open' → 'Open anyway', or run `xattr -d com.apple.quarantine '<path-to-app>'` to clear the Gatekeeper flag."
4. Exit 0.

### Error model

- `pipefail` + `set -e` make any failed step abort the script.
- `ERR` trap prints the failing line number for fast diagnosis. No stack traces.
- Each pre-build failure (missing Tesseract, unsupported OS) exits with a one-line actionable message.
- No retries on transient failures — let the user re-run.

## Files modified

### New: `scripts/build-local.sh`

Per the flow above.

### Modified: `app/src-tauri/tauri.conf.json`

Remove the `bundle.windows` block entirely:

```diff
   "bundle": {
     "active": true,
     "targets": "all",
     "createUpdaterArtifacts": true,
     "icon": ["icons/icon.ico", "icons/icon.png"],
     "resources": {
       "resources/tesseract/**/*": "tesseract/"
-    },
-    "windows": {
-      "signCommand": "sign code artifact-signing -ase https://eus.codesigning.azure.net -asa HartyeTech -ascp snapper-keeper %1"
     }
   },
```

### Modified: `.github/workflows/release.yml`

Delete the `Strip signCommand from tauri.conf.json (Windows build)` step (release.yml:110–117). The justification comment block above the step is also removed. The subsequent `Build Tauri app (unsigned)` step's `--config '{"bundle":{"createUpdaterArtifacts":false}}'` argument remains; its inline comment is reworded to drop the now-stale reference to the deleted `jq` step.

### Modified: `package.json` (root)

```diff
   "scripts": {
     ...
+    "build:local": "bash scripts/build-local.sh",
     ...
   }
```

### Modified: `README.md`

Replace the existing "Build a release bundle" section (lines 75–81) with a "Build a local installer (unsigned)" section that documents:

- The `pnpm build:local` command.
- That output is `.app` + `.dmg` on macOS and a `*-setup.exe` (with bundled Tesseract) on Windows.
- The exact ways local diverges from production: no Authenticode/codesign/notarize, no updater payload. Everything else is identical.
- Per-OS instructions for installing the unsigned artifact (SmartScreen on Windows, Gatekeeper on macOS).
- That Linux is not a supported installer target; use `pnpm tauri dev` instead.

### Modified: `docs/release-signing.md`

Add a "Where signing lives in the build system" section near the top, before the Ed25519 section, stating that `app/src-tauri/tauri.conf.json` contains no signing commands and that production Windows signing happens entirely in the `sign-win-x64` job via direct `sign code` invocation, not via Tauri's `signCommand` hook. This is the explanation any future reader needs to understand why the base config is intentionally signing-free.

## Validation plan

The CI behavior change must be proven a no-op before the next real release tag, and the new local script must run on both target OSes before merge.

### Pre-merge

1. **Local script — Windows.** Run `pnpm build:local` on the interactive Win11 desktop session (per CLAUDE.md, Windows builds require an interactive station). Confirm:
   - Tesseract resolver finds the existing install.
   - `*-setup.exe` lands under `target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
   - SHA-256 + size are printed.
   - Copied Tesseract files in `app/src-tauri/resources/tesseract/` are removed after the script exits; only `.placeholder` remains.
   - Installing + running the produced installer launches Snapper Keeper, captures a screen, OCR returns text (proves bundled Tesseract works).
2. **Local script — macOS.** Run `pnpm build:local` on macOS. Confirm:
   - `.app` + `.dmg` land under `target/<triple>/release/bundle/{macos,dmg}/`.
   - Opening the `.dmg`, dragging `.app` to Applications, right-click → Open clears Gatekeeper.
   - Launched app works (OCR via brew-installed tesseract).
   - SHA-256 + size are printed for both artifacts.
3. **Local script — Linux.** Run on Linux or WSL. Confirm clean exit with the supported-platforms message.
4. **CI green path.** Push the branch; verify `build-app` (`.github/workflows/ci.yml`) still passes on all three OSes.
5. **Dry-run the release flow.** Cut a throwaway prerelease tag (e.g. `v0.1.3-localbuild-test`) from the feature branch. Per `docs/release-signing.md`, the `releases/latest/` updater pointer ignores prereleases, so existing clients are unaffected. The full pipeline runs end-to-end and you confirm `*-setup.exe` is still Authenticode-signed at the publish step. Delete the GitHub Release + tag after verification.

### Post-merge

When cutting the next real release tag, watch the `sign-win-x64` job's `Authenticode-sign the installer` step. `Get-AuthenticodeSignature` should report `Valid` with the HartyeTech signer subject — same as before. If anything regresses, the fix is a one-line revert restoring `bundle.windows.signCommand` to the base config; no other rollback needed.

### Explicitly not validated

- Cross-architecture builds locally — each contributor builds for their native arch.
- Signed local builds — out of scope.
- Auto-updater behavior on a locally-built install. Local installs have no `.sig` payload, but they can verify the published `latest.json` against the embedded pubkey, against signed prod releases. Updater UX from an unsigned local install: it discovers prod updates and offers to apply them; that is expected and a fine smoke-test path.

## Risks

- **Tauri overlay precedence.** The inline `--config '{"bundle":{"createUpdaterArtifacts":false}}'` overlay is already proven by the CI build job; reusing it locally is low risk.
- **Tesseract resolver missing a contributor's install.** Mitigated by `SNK_TESSERACT_PATH` override and an actionable error message pointing at the README.
- **Removing `bundle.windows.signCommand` breaks Windows signing in CI.** Mitigated by the dry-run prerelease tag step in the validation plan, and by the fact that the actual `sign code` invocation in `sign-win-x64` does not reference `signCommand`. Revert is one line if it ever fails.
- **Working-tree pollution on Windows if the script is killed before its EXIT trap runs.** The bundled Tesseract files are already gitignored at `.gitignore:23-24`, so `git status` stays clean. A subsequent re-run of `pnpm build:local` overwrites whatever is there. The `.placeholder` file is robust to this because the script never deletes it.

## Open questions

None at design time. Implementation-time decisions (exact wording of the README section, the SHA-256 print format, whether to include `.app` size or only `.dmg` size on macOS) will be made by the writing-plans / implementation phase.
