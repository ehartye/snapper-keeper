# Phase 7: Signing, Notarization, Auto-Updater & Release Pipeline

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Ship signed, notarized installers via GitHub Releases with Tauri auto-updater support so users get seamless over-the-air updates.

**Architecture:** `snk-updater` is a thin Tauri plugin wrapping `tauri-plugin-updater`. It checks for updates on launch and every 24 hours, surfaces status in the tray menu, and prompts "Restart to update" — never auto-applies. Code signing (macOS Apple Developer ID + Windows cert) and notarization happen in a GitHub Actions release workflow triggered on `v*` tags. The workflow builds platform-specific artifacts, signs them, notarizes macOS bundles, uploads to GitHub Releases, and generates `latest.json` with Ed25519 signatures for the updater.

**Tech Stack:** Rust (tauri-plugin-updater, tokio, serde), TypeScript, GitHub Actions, `notarytool` (macOS), `signtool`/`AzureSignTool` (Windows), Tauri CLI (`tauri signer`)

**Phase 7 scope:**
- `snk-updater` crate (Tauri plugin wrapping `tauri-plugin-updater`)
- Tauri updater configuration (`tauri.conf.json` endpoint + pubkey)
- Tray menu "Check for updates" item + status feedback
- GitHub Actions release workflow (`release.yml`, `v*` tag trigger)
- macOS signing + notarization in CI
- Windows code signing in CI
- Ed25519 key generation instructions + `latest.json` generation
- Capability + permission wiring for the updater plugin

**Out of scope:** Store distribution, paid features, analytics, beta channel, E2E tests (manual release checklist covers update flow).

**Pre-flight:**
- Phases 1–6 merged to `main`
- `cargo test --workspace` passes
- `pnpm typecheck` clean (modulo pre-existing annotate/clipboard module declaration issues)
- `pnpm lint` clean

---

### Task 1: Generate Ed25519 key pair and document secrets setup

**Files:**
- Create: `docs/release-signing.md`

**Step 1: Document the key generation process**

Create `docs/release-signing.md` with instructions for generating the Tauri updater signing key pair and configuring GitHub Actions secrets:

```markdown
# Release Signing Setup

## Ed25519 updater key pair

The Tauri updater uses Ed25519 signatures to verify update payloads. The private key signs `latest.json` during CI; the public key is embedded in `tauri.conf.json`.

### Generate the key pair

```bash
pnpm tauri signer generate -w ~/.tauri/snapper-keeper.key
```

This creates:
- `~/.tauri/snapper-keeper.key` — private key (password-protected)
- `~/.tauri/snapper-keeper.key.pub` — public key

### GitHub Actions secrets

Add these secrets to the repository (`Settings > Secrets and variables > Actions`):

| Secret | Value | Used by |
|--------|-------|---------|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `~/.tauri/snapper-keeper.key` | `release.yml` — signs update bundles |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password used during key generation | `release.yml` — unlocks private key |

### macOS signing secrets

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email for `notarytool` |
| `APPLE_PASSWORD` | App-specific password for `notarytool` |
| `APPLE_TEAM_ID` | 10-character team ID |

### Windows signing secrets

| Secret | Value |
|--------|-------|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` code-signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` |
```

**Step 2: Commit**

```
git add docs/release-signing.md
git commit -m "docs: add release signing setup guide"
```

---

### Task 2: Create `snk-updater` crate scaffold

**Files:**
- Create: `crates/snk-updater/Cargo.toml`
- Create: `crates/snk-updater/build.rs`
- Create: `crates/snk-updater/src/lib.rs`
- Create: `crates/snk-updater/src/plugin.rs`
- Create: `crates/snk-updater/permissions/default.toml`
- Modify: `Cargo.toml` (workspace root)
- Modify: `app/src-tauri/Cargo.toml`

**Step 1: Create `crates/snk-updater/Cargo.toml`**

```toml
[package]
name = "snk-updater"
version = "0.0.1"
links = "snk-updater"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[build-dependencies]
tauri-plugin = { workspace = true }

[dependencies]
tauri.workspace = true
tauri-plugin-updater = "2"
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
```

**Step 2: Create `crates/snk-updater/build.rs`**

```rust
const COMMANDS: &[&str] = &["check_for_update", "get_update_status"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 3: Create `crates/snk-updater/src/lib.rs`**

```rust
//! snk-updater — auto-update check, download, and restart prompt.

pub mod plugin;

pub use plugin::init;
```

**Step 4: Create `crates/snk-updater/src/plugin.rs` with stub commands**

```rust
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available { version: String },
    Downloading { percent: f32 },
    Ready { version: String },
    Error { detail: String },
}

pub struct UpdaterState {
    status: Mutex<UpdateStatus>,
}

impl UpdaterState {
    fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
        }
    }

    fn set_status(&self, s: UpdateStatus) {
        if let Ok(mut lock) = self.status.lock() {
            *lock = s;
        }
    }

    fn get_status(&self) -> UpdateStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or(UpdateStatus::Idle)
    }
}

#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    do_update_check(app).await
}

#[tauri::command]
pub fn get_update_status<R: Runtime>(app: AppHandle<R>) -> UpdateStatus {
    app.state::<UpdaterState>().get_status()
}

async fn do_update_check<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    let state = app.state::<UpdaterState>();
    state.set_status(UpdateStatus::Checking);
    let _ = app.emit("updater:status-changed", UpdateStatus::Checking);

    let updater = app.updater().map_err(|e| format!("updater init: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            info!(%version, "update available");
            let status = UpdateStatus::Available {
                version: version.clone(),
            };
            state.set_status(status.clone());
            let _ = app.emit("updater:status-changed", status.clone());

            let dl_handle = app.app_handle().clone();
            let done_handle = app.app_handle().clone();
            let err_handle = app.app_handle().clone();
            tokio::spawn(async move {
                let mut downloaded: u64 = 0;
                match update
                    .download_and_install(
                        |chunk, content_length| {
                            downloaded += chunk as u64;
                            let percent = content_length
                                .map(|cl| (downloaded as f32 / cl as f32) * 100.0)
                                .unwrap_or(0.0);
                            let status = UpdateStatus::Downloading { percent };
                            dl_handle.state::<UpdaterState>().set_status(status.clone());
                            let _ = dl_handle.emit("updater:status-changed", status);
                        },
                        || {
                            let status = UpdateStatus::Ready {
                                version: version.clone(),
                            };
                            done_handle.state::<UpdaterState>().set_status(status.clone());
                            let _ = done_handle.emit("updater:status-changed", status);
                            info!(%version, "update ready — restart to apply");
                        },
                    )
                    .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        let status = UpdateStatus::Error {
                            detail: e.to_string(),
                        };
                        err_handle.state::<UpdaterState>().set_status(status.clone());
                        let _ = err_handle.emit("updater:status-changed", status);
                        error!(error = %e, "update download failed");
                    }
                }
            });

            Ok(status)
        }
        Ok(None) => {
            info!("no update available");
            state.set_status(UpdateStatus::Idle);
            let _ = app.emit("updater:status-changed", UpdateStatus::Idle);
            Ok(UpdateStatus::Idle)
        }
        Err(e) => {
            warn!(error = %e, "update check failed");
            let status = UpdateStatus::Error {
                detail: e.to_string(),
            };
            state.set_status(status.clone());
            let _ = app.emit("updater:status-changed", status.clone());
            Ok(status)
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-updater")
        .invoke_handler(tauri::generate_handler![check_for_update, get_update_status])
        .setup(|app, _api| {
            app.manage(UpdaterState::new());

            let handle = app.app_handle().clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Err(e) = do_update_check(handle.clone()).await {
                    warn!(error = %e, "startup update check failed");
                }

                let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    if let Err(e) = do_update_check(handle.clone()).await {
                        warn!(error = %e, "periodic update check failed");
                    }
                }
            });

            Ok(())
        })
        .build()
}
```

**Step 5: Create `crates/snk-updater/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-updater plugin"
permissions = ["allow-check-for-update", "allow-get-update-status"]
```

**Step 6: Add `snk-updater` to workspace root `Cargo.toml`**

Add `"crates/snk-updater"` to the `members` array, after `"crates/snk-ocr"`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/snk-library",
    "crates/snk-hotkeys",
    "crates/snk-capture",
    "crates/snk-annotate",
    "crates/snk-clipboard",
    "crates/snk-ocr",
    "crates/snk-updater",
    "app/src-tauri",
]
```

**Step 7: Add `snk-updater` + `tauri-plugin-updater` deps to `app/src-tauri/Cargo.toml`**

Add to `[dependencies]`:

```toml
snk-updater = { path = "../../crates/snk-updater" }
```

The `tauri` dependency does NOT need an `updater` feature — in Tauri 2, updater functionality lives entirely in `tauri-plugin-updater`. Leave the existing features as-is:

```toml
tauri = { version = "2", features = ["tray-icon", "protocol-asset"] }
```

**Step 8: Verify it compiles**

Run: `cargo check -p snk-updater`
Expected: compiles (warnings OK, no errors)

**Step 9: Commit**

```
git add crates/snk-updater/Cargo.toml crates/snk-updater/build.rs crates/snk-updater/src/lib.rs crates/snk-updater/src/plugin.rs crates/snk-updater/permissions/default.toml Cargo.toml app/src-tauri/Cargo.toml
git commit -m "feat(updater): scaffold snk-updater crate with update check + periodic timer"
```

---

### Task 3: Add updater configuration to `tauri.conf.json`

**Files:**
- Modify: `app/src-tauri/tauri.conf.json`

**Step 1: Add the `plugins.updater` section to `tauri.conf.json`**

Add a top-level `"plugins"` key after the `"bundle"` section:

```json
"plugins": {
  "updater": {
    "pubkey": "REPLACE_WITH_PUBLIC_KEY",
    "endpoints": [
      "https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json"
    ]
  }
}
```

**Note:** The `pubkey` value is a placeholder. It must be replaced with the actual Ed25519 public key after the user generates the key pair per Task 1's docs. The `endpoints` URL uses the GitHub Releases convention — the actual repo owner/name should match the real repository.

**Step 2: Verify config is still valid JSON**

Run: `cargo check -p snapper-keeper-app`
Expected: compiles (the updater endpoint is only fetched at runtime, not build time)

**Step 3: Commit**

```
git add app/src-tauri/tauri.conf.json
git commit -m "feat(updater): add updater plugin config with endpoint + pubkey placeholder"
```

---

### Task 4: Wire `snk-updater` into the app + capabilities

**Files:**
- Modify: `app/src-tauri/src/main.rs`
- Modify: `app/src-tauri/capabilities/default.json`

**Step 1: Register `snk-updater` plugin in `main.rs`**

Add `tauri_plugin_updater` and `snk_updater` to the plugin chain. The updater plugin must be registered **before** `snk_updater` because `snk_updater` calls `app.updater()` which requires the upstream plugin.

In `main.rs`, add to the imports section:

```rust
// No new use imports needed — snk_updater::init() is the only entry point
```

Add two `.plugin()` calls after the existing plugin chain, before `.setup()`:

```rust
.plugin(tauri_plugin_updater::Builder::new().build())
.plugin(snk_updater::init())
```

The full plugin chain becomes:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(snk_library::init())
    .plugin(snk_hotkeys::init())
    .plugin(snk_capture::init())
    .plugin(snk_annotate::init())
    .plugin(snk_clipboard::init())
    .plugin(snk_ocr::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(snk_updater::init())
    .setup(|app| {
        // ...existing setup code...
    })
```

**Step 2: Add `snk-updater` permission to capabilities**

In `app/src-tauri/capabilities/default.json`, add `"settings"` to the windows array and `"snk-updater:default"` to the permissions array:

```json
{
  "$schema": "https://schema.tauri.app/capabilities/2",
  "identifier": "default",
  "description": "Default permissions for the library window",
  "windows": ["library", "capture-overlay", "capture-toolbar", "annotate", "clipboard-popup", "settings"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:event:default",
    "core:path:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",
    "snk-library:default",
    "snk-capture:default",
    "snk-annotate:default",
    "snk-clipboard:default",
    "snk-ocr:default",
    "snk-updater:default"
  ]
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p snapper-keeper-app`
Expected: compiles without errors

**Step 4: Commit**

```
git add app/src-tauri/src/main.rs app/src-tauri/capabilities/default.json
git commit -m "feat(updater): wire snk-updater plugin into app + capabilities"
```

---

### Task 5: Add "Check for updates" to tray menu

**Files:**
- Modify: `app/src-tauri/src/main.rs`

**Step 1: Add the tray menu item**

In the `setup` closure in `main.rs`, add a "Check for updates" menu item after the `settings` item and before `quit`:

```rust
let check_update = MenuItem::with_id(
    app,
    "tray:check-update",
    "Check for updates",
    true,
    None::<&str>,
)?;
```

Update the `Menu::with_items` call to include it:

```rust
let menu = Menu::with_items(
    app,
    &[
        &capture_region,
        &capture_window,
        &capture_screen,
        &capture_timed,
        &clipboard_hist,
        &sep,
        &open_lib,
        &settings,
        &check_update,
        &quit,
    ],
)?;
```

**Step 2: Handle the menu event**

In the `.on_menu_event()` closure, add a match arm before the `"tray:quit"` arm. Use `tauri::async_runtime::spawn` because tray menu event handlers run outside the tokio context:

```rust
"tray:check-update" => {
    let handle = app.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let _ = snk_updater::plugin::check_for_update(handle).await;
    });
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p snapper-keeper-app`
Expected: compiles without errors

**Step 4: Commit**

```
git add app/src-tauri/src/main.rs
git commit -m "feat(updater): add 'Check for updates' tray menu item"
```

---

### Task 6: Create TS bindings for `snk-updater`

**Files:**
- Create: `packages/snk-updater/package.json`
- Create: `packages/snk-updater/tsconfig.json`
- Create: `packages/snk-updater/src/types.ts`
- Create: `packages/snk-updater/src/index.ts`

**Step 1: Create `packages/snk-updater/package.json`**

```json
{
  "name": "@snk/updater",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "main": "src/index.ts",
  "types": "src/index.ts",
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  }
}
```

**Step 2: Create `packages/snk-updater/tsconfig.json`**

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "rootDir": "src",
    "outDir": "dist"
  },
  "include": ["src"]
}
```

**Step 3: Create `packages/snk-updater/src/types.ts`**

```typescript
export type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'available'; version: string }
  | { kind: 'downloading'; percent: number }
  | { kind: 'ready'; version: string }
  | { kind: 'error'; detail: string };
```

Wait — the Rust `UpdateStatus` enum uses `#[serde(rename_all = "kebab-case")]` and it's not a tagged enum (no `#[serde(tag = "kind")]`). It's a Rust enum with variants that have named fields. By default, serde serializes this as an externally tagged enum: `{ "available": { "version": "1.0" } }` or just `"idle"` for unit variants.

Let me reconsider the serialization. Looking at the Rust code in Task 2, `UpdateStatus` uses `#[serde(rename_all = "kebab-case")]`. Without an explicit tag representation, serde uses externally tagged by default:
- `Idle` → `"idle"`
- `Checking` → `"checking"`
- `Available { version }` → `{ "available": { "version": "1.0" } }`
- `Downloading { percent }` → `{ "downloading": { "percent": 50.0 } }`
- `Ready { version }` → `{ "ready": { "version": "1.0" } }`
- `Error { detail }` → `{ "error": { "detail": "..." } }`

But per CLAUDE.md, the project convention is `#[serde(tag = "kind")]` for IPC error enums. Let's follow that convention and add `#[serde(tag = "kind")]` to the Rust enum in Task 2. Update the Rust `UpdateStatus` definition to include `#[serde(tag = "kind", rename_all = "kebab-case")]`.

**PLAN CORRECTION (Task 2):** The `UpdateStatus` enum in `crates/snk-updater/src/plugin.rs` must have `#[serde(tag = "kind", rename_all = "kebab-case")]` so it serializes as `{ "kind": "available", "version": "1.0" }`, matching the project's IPC convention.

Now the TS types are:

```typescript
export type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'available'; version: string }
  | { kind: 'downloading'; percent: number }
  | { kind: 'ready'; version: string }
  | { kind: 'error'; detail: string };
```

**Step 4: Create `packages/snk-updater/src/index.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

import type { UpdateStatus } from './types';

export * from './types';

export function checkForUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|check_for_update');
}

export function getUpdateStatus(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|get_update_status');
}
```

**Step 5: Run `pnpm install` from workspace root to link the new package**

Run: `pnpm install`
Expected: `@snk/updater` linked into the workspace

**Step 6: Verify TypeScript compiles**

Run: `npx tsc -p packages/snk-updater/tsconfig.json --noEmit`
Expected: no errors

**Step 7: Commit**

```
git add packages/snk-updater/package.json packages/snk-updater/tsconfig.json packages/snk-updater/src/types.ts packages/snk-updater/src/index.ts pnpm-lock.yaml
git commit -m "feat(updater): add @snk/updater TS bindings package"
```

---

### Task 7: Create GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Create the release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-release:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            label: macOS-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            label: macOS-x64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            label: Windows-x64

    runs-on: ${{ matrix.os }}
    permissions:
      contents: write

    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v3
        with:
          version: 9

      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Install Linux deps
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev

      - run: pnpm install --frozen-lockfile

      # macOS: import signing certificate
      - name: Import Apple certificate
        if: runner.os == 'macOS'
        env:
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        run: |
          echo "$APPLE_CERTIFICATE" | base64 --decode > certificate.p12
          security create-keychain -p actions build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p actions build.keychain
          security import certificate.p12 -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
          security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k actions build.keychain
          rm certificate.p12

      # Windows: import code-signing certificate (must be before build)
      - name: Import Windows certificate
        if: runner.os == 'Windows'
        env:
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
        shell: powershell
        run: |
          $certBytes = [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE)
          [IO.File]::WriteAllBytes("certificate.pfx", $certBytes)

      # Build with Tauri CLI — handles signing, bundling, and updater artifacts
      - name: Build Tauri app
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          # macOS signing
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          # Windows signing
          TAURI_WINDOWS_SIGN_COMMAND: ${{ runner.os == 'Windows' && format('signtool sign /fd SHA256 /t http://timestamp.digicert.com /f certificate.pfx /p {0} "%1"', secrets.WINDOWS_CERTIFICATE_PASSWORD) || '' }}
        run: |
          pnpm tauri build --target ${{ matrix.target }}

      # macOS: notarize the .app bundle
      - name: Notarize macOS app
        if: runner.os == 'macOS'
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: |
          DMG_PATH=$(find app/src-tauri/target/${{ matrix.target }}/release/bundle/dmg -name "*.dmg" | head -1)
          if [ -n "$DMG_PATH" ]; then
            xcrun notarytool submit "$DMG_PATH" \
              --apple-id "$APPLE_ID" \
              --password "$APPLE_PASSWORD" \
              --team-id "$APPLE_TEAM_ID" \
              --wait
            xcrun stapler staple "$DMG_PATH"
          fi

      # Upload artifacts for the release job
      - name: Upload build artifacts
        uses: actions/upload-artifact@v4
        with:
          name: artifacts-${{ matrix.label }}
          path: |
            app/src-tauri/target/${{ matrix.target }}/release/bundle/dmg/*.dmg
            app/src-tauri/target/${{ matrix.target }}/release/bundle/macos/*.app.tar.gz
            app/src-tauri/target/${{ matrix.target }}/release/bundle/macos/*.app.tar.gz.sig
            app/src-tauri/target/${{ matrix.target }}/release/bundle/nsis/*.exe
            app/src-tauri/target/${{ matrix.target }}/release/bundle/nsis/*.nsis.zip
            app/src-tauri/target/${{ matrix.target }}/release/bundle/nsis/*.nsis.zip.sig
          if-no-files-found: error

      - name: Clean up certificates
        if: always()
        shell: bash
        run: |
          rm -f certificate.p12 certificate.pfx

  publish-release:
    needs: build-and-release
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - name: List artifacts
        run: find artifacts -type f | sort

      - name: Generate latest.json
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          NOTES="Release $VERSION"
          PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

          # Collect platform entries
          PLATFORMS='{}'

          # macOS aarch64
          MAC_ARM_TAR=$(find artifacts -name "*.app.tar.gz" -path "*aarch64*" | head -1)
          MAC_ARM_SIG=$(find artifacts -name "*.app.tar.gz.sig" -path "*aarch64*" | head -1)
          if [ -n "$MAC_ARM_TAR" ] && [ -n "$MAC_ARM_SIG" ]; then
            SIG=$(cat "$MAC_ARM_SIG")
            URL="https://github.com/${{ github.repository }}/releases/download/${GITHUB_REF_NAME}/$(basename "$MAC_ARM_TAR")"
            PLATFORMS=$(echo "$PLATFORMS" | jq --arg url "$URL" --arg sig "$SIG" '. + {"darwin-aarch64": {"url": $url, "signature": $sig}}')
          fi

          # macOS x86_64
          MAC_X64_TAR=$(find artifacts -name "*.app.tar.gz" -path "*x86_64*" | head -1)
          MAC_X64_SIG=$(find artifacts -name "*.app.tar.gz.sig" -path "*x86_64*" | head -1)
          if [ -n "$MAC_X64_TAR" ] && [ -n "$MAC_X64_SIG" ]; then
            SIG=$(cat "$MAC_X64_SIG")
            URL="https://github.com/${{ github.repository }}/releases/download/${GITHUB_REF_NAME}/$(basename "$MAC_X64_TAR")"
            PLATFORMS=$(echo "$PLATFORMS" | jq --arg url "$URL" --arg sig "$SIG" '. + {"darwin-x86_64": {"url": $url, "signature": $sig}}')
          fi

          # Windows x86_64
          WIN_ZIP=$(find artifacts -name "*.nsis.zip" | head -1)
          WIN_SIG=$(find artifacts -name "*.nsis.zip.sig" | head -1)
          if [ -n "$WIN_ZIP" ] && [ -n "$WIN_SIG" ]; then
            SIG=$(cat "$WIN_SIG")
            URL="https://github.com/${{ github.repository }}/releases/download/${GITHUB_REF_NAME}/$(basename "$WIN_ZIP")"
            PLATFORMS=$(echo "$PLATFORMS" | jq --arg url "$URL" --arg sig "$SIG" '. + {"windows-x86_64": {"url": $url, "signature": $sig}}')
          fi

          jq -n \
            --arg version "$VERSION" \
            --arg notes "$NOTES" \
            --arg pub_date "$PUB_DATE" \
            --argjson platforms "$PLATFORMS" \
            '{version: $version, notes: $notes, pub_date: $pub_date, platforms: $platforms}' \
            > latest.json

          echo "Generated latest.json:"
          cat latest.json

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          draft: false
          prerelease: false
          generate_release_notes: true
          files: |
            artifacts/**/*.dmg
            artifacts/**/*.app.tar.gz
            artifacts/**/*.app.tar.gz.sig
            artifacts/**/*.exe
            artifacts/**/*.nsis.zip
            artifacts/**/*.nsis.zip.sig
            latest.json
```

**Step 2: Verify YAML syntax**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`
If Python/PyYAML not available, a quick visual check is sufficient. The CI will validate on push.

**Step 3: Commit**

```
git add .github/workflows/release.yml
git commit -m "ci: add release workflow with signing, notarization, and auto-updater manifest"
```

---

### Task 8: Add `tauri-plugin-updater` to workspace dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add `tauri-plugin-updater` to workspace dependencies**

In the `[workspace.dependencies]` section, add:

```toml
tauri-plugin-updater = "2"
```

**Step 2: Update `crates/snk-updater/Cargo.toml` to use workspace dep**

Change the `tauri-plugin-updater` dependency to use workspace inheritance:

```toml
tauri-plugin-updater.workspace = true
```

Wait — per CLAUDE.md, the `tauri` dep must NOT use `workspace = true` in `app/src-tauri/Cargo.toml` because it blocks `tauri-build` auto-rewriting features. But `tauri-plugin-updater` is a different dependency and doesn't have this restriction. However, to keep things simple and consistent, let's use the workspace dep for `tauri-plugin-updater` in the snk-updater crate since it doesn't need special feature handling.

**Step 3: Verify it compiles**

Run: `cargo check -p snk-updater`
Expected: compiles

**Step 4: Commit**

```
git add Cargo.toml crates/snk-updater/Cargo.toml
git commit -m "chore: add tauri-plugin-updater to workspace dependencies"
```

**NOTE:** This task should be done as part of Task 2 to avoid a broken intermediate state. When implementing, merge this into Task 2's step 6 (workspace Cargo.toml changes). The task is listed separately for clarity but should be committed together with Task 2.

---

### Task 9: Add `tauri-plugin-updater` dependency to `app/src-tauri/Cargo.toml`

**Files:**
- Modify: `app/src-tauri/Cargo.toml`

**Step 1: Add `tauri-plugin-updater` to app dependencies**

The app binary needs `tauri-plugin-updater` directly because it calls `tauri_plugin_updater::Builder::new().build()` in `main.rs`.

Add to `[dependencies]` in `app/src-tauri/Cargo.toml`:

```toml
tauri-plugin-updater = "2"
```

**NOTE:** Do NOT use `workspace = true` here — follow the same pattern as `tauri-plugin-global-shortcut.workspace = true`. Actually, `tauri-plugin-global-shortcut` DOES use workspace inheritance in the app Cargo.toml. So `tauri-plugin-updater` can too, as long as it's in the workspace deps (Task 8). Use:

```toml
tauri-plugin-updater.workspace = true
```

**Step 2: Verify it compiles**

Run: `cargo check -p snapper-keeper-app`
Expected: compiles

**Step 3: Commit**

This should be committed as part of Task 4 (wiring the plugin into main.rs) since the import won't compile without the dependency. When implementing, fold this into Task 4's commit.

---

### Task 10: Update CI workflow for updater Linux deps

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add `libssl-dev` to Linux dependencies**

The `tauri-plugin-updater` crate depends on `reqwest` which needs OpenSSL headers on Linux. Add `libssl-dev` to both the `rust-test` and `build-app` Linux dep install steps.

In the `rust-test` job, update the install step:

```yaml
- name: Install Linux Tauri deps
  run: |
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev libxdo-dev libssl-dev
```

In the `build-app` job, update the install step:

```yaml
- name: Install Linux deps
  if: matrix.os == 'ubuntu-latest'
  run: |
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev libssl-dev
```

**Step 2: Verify CI config syntax**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Or visual check.

**Step 3: Commit**

```
git add .github/workflows/ci.yml
git commit -m "ci: add libssl-dev for tauri-plugin-updater reqwest dependency"
```

---

### Task 11: Rust unit tests for `snk-updater`

**Files:**
- Modify: `crates/snk-updater/src/plugin.rs`

**Step 1: Add unit tests for `UpdaterState`**

Append tests to `crates/snk-updater/src/plugin.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_status_is_idle() {
        let state = UpdaterState::new();
        assert_eq!(state.get_status(), UpdateStatus::Idle);
    }

    #[test]
    fn set_and_get_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Checking);
        assert_eq!(state.get_status(), UpdateStatus::Checking);
    }

    #[test]
    fn set_available_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Available {
            version: "1.2.3".to_string(),
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Available {
                version: "1.2.3".to_string()
            }
        );
    }

    #[test]
    fn set_downloading_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Downloading { percent: 42.5 });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Downloading { percent: 42.5 }
        );
    }

    #[test]
    fn set_error_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Error {
            detail: "network timeout".to_string(),
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Error {
                detail: "network timeout".to_string()
            }
        );
    }

    #[test]
    fn status_transitions() {
        let state = UpdaterState::new();
        assert_eq!(state.get_status(), UpdateStatus::Idle);

        state.set_status(UpdateStatus::Checking);
        assert_eq!(state.get_status(), UpdateStatus::Checking);

        state.set_status(UpdateStatus::Available {
            version: "2.0.0".to_string(),
        });
        state.set_status(UpdateStatus::Downloading { percent: 50.0 });
        state.set_status(UpdateStatus::Ready {
            version: "2.0.0".to_string(),
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Ready {
                version: "2.0.0".to_string()
            }
        );
    }

    #[test]
    fn serde_roundtrip_unit_variants() {
        let idle = UpdateStatus::Idle;
        let json = serde_json::to_string(&idle).unwrap();
        assert!(json.contains("\"kind\":\"idle\""));
        let parsed: UpdateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UpdateStatus::Idle);
    }

    #[test]
    fn serde_roundtrip_data_variants() {
        let available = UpdateStatus::Available {
            version: "3.0.0".to_string(),
        };
        let json = serde_json::to_string(&available).unwrap();
        assert!(json.contains("\"kind\":\"available\""));
        assert!(json.contains("\"version\":\"3.0.0\""));
        let parsed: UpdateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, available);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p snk-updater`
Expected: 8 tests pass

**Step 3: Commit**

```
git add crates/snk-updater/src/plugin.rs
git commit -m "test(updater): add unit tests for UpdaterState and serde roundtrip"
```

---

### Task 12: Integration verification and `rustfmt` sweep

**Files:**
- Potentially any files with formatting issues

**Step 1: Run `cargo fmt` across workspace**

Run: `cargo fmt --all`

**Step 2: Run full test suite**

Run: `cargo test --workspace --exclude snapper-keeper-app`
Expected: all tests pass (existing + new snk-updater tests)

**Step 3: Run clippy**

Run: `cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings`
Expected: no warnings

**Step 4: Run TypeScript checks**

Run: `pnpm typecheck`
Expected: clean (modulo pre-existing annotate/clipboard module declaration issues)

Run: `pnpm lint`
Expected: clean

**Step 5: Build the app**

Run: `cargo build -p snapper-keeper-app`
Expected: compiles successfully

**Step 6: Commit any formatting changes**

```
git add -u
git commit -m "chore: fmt sweep after phase 7"
```

Only commit if there are actual changes. If `cargo fmt` made no changes, skip this step.

---

## Task dependency graph

```
Task 1 (docs)          — independent
Task 2 (crate) + 8     — Task 8 merged into Task 2
Task 3 (config)        — after Task 2
Task 4 (wiring) + 9    — after Task 2, Task 3; Task 9 merged into Task 4
Task 5 (tray menu)     — after Task 4
Task 6 (TS bindings)   — after Task 2
Task 7 (release.yml)   — independent
Task 10 (CI fix)       — independent
Task 11 (tests)        — after Task 2
Task 12 (verification) — after all tasks
```

**Parallelization:** Tasks 1, 7, 10 can run in parallel. Tasks 2+8 and 6 can run in parallel after 1/7/10 or independently. Task 3 follows Task 2. Task 4+9 follows Task 3. Task 5 follows Task 4. Task 11 follows Task 2. Task 12 is last.

## Corrections from plan self-review

1. **Task 2 serde tag:** Added `#[serde(tag = "kind", rename_all = "kebab-case")]` to `UpdateStatus` to match the project's IPC convention (CLAUDE.md: "The discriminator tag in serde is `"kind"`").

2. **Task 8 merged into Task 2:** To avoid a broken intermediate state where `snk-updater` can't compile because `tauri-plugin-updater` isn't in the workspace deps, Task 8's workspace dependency addition is folded into Task 2's commit.

3. **Task 9 merged into Task 4:** The app binary needs `tauri-plugin-updater` to compile the `main.rs` changes from Task 4. Folded into the same commit.

4. **`total` variable in Task 2:** Removed unused `total` binding in the download closure.

5. **`UpdaterState` Clone derive:** `UpdaterState` contains a `Mutex` and cannot derive `Clone`. The `state2` binding in the spawn closure needs to be an `Arc`. Changed `UpdaterState` to be wrapped in `Arc` via Tauri's `manage()` (which already wraps in `Arc` internally). The `app.state::<UpdaterState>()` returns a `State<UpdaterState>` which derefs to `&UpdaterState`. For the spawned closure, clone the `AppHandle` instead and re-access state inside the closure.

6. **Windows certificate import timing:** Moved the Windows certificate import step before the build step in the release workflow so `signtool` can find it during the Tauri build.
