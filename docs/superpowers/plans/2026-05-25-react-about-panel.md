# React About Panel (PR B) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:team-driven-development to implement this plan.

**Goal:** Add a Settings → About section with version + paths + updater status, closing #36. Resolve #62 by deleting the unused `@snk/ocr` TS package (its sole export is a stub binding to a stub Rust command; About has no use for it).

**Architecture:**
- About is a new `<SettingsSection title="About">` appended to `app/src/windows/settings/SettingsWindow.tsx` (not a separate window — matches the #36 issue text and reuses PR A's shared primitives).
- Git short-SHA and updater-pubkey fingerprint are injected at build time via Vite `define` so no new Rust commands are needed for those values.
- `tauri-plugin-opener` (Tauri 2 canonical) handles both filesystem-path opening and HTTPS URL opening from the same TS API.
- snk-updater gains `last_check_at` state + matching command, and a small `restart_app` command for the post-ready restart flow.
- @snk/ocr deletion is one task: remove the directory + drop the vitest alias.

**Tech Stack:** React 18, TypeScript strict, Tauri 2, `tauri-plugin-opener`, Vite `define` at build time, Vitest + RTL.

**Spec:** Per #36 (issue body verbatim) and the cluster design at `docs/superpowers/specs/2026-05-25-react-cluster-design.md` with three scope clarifications captured in this plan (About is a Settings section, not a window; @snk/ocr deletes; OCR engine section drops with it).

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel/`
**Branch:** `feat/react-about-panel` (off `origin/main` after PR A merged)
**Baseline:** 212/212 TS tests passing.

---

## Conventions

- Conventional Commits: `feat(ui):`, `feat(updater):`, `chore(deps):`, `chore:`, `test(ui):`, `refactor:`.
- Staging: `git add <explicit-paths>`, NEVER `git add .` or `-A`.
- One task = one commit.
- TDD where tests are meaningful; for plumbing tasks (Vite define, deps add, package deletion) verification is "the suite still passes."
- No comments unless the WHY is non-obvious.

## Dependency graph

```
T1 (Vite define inject)        ┐
T2 (tauri-plugin-opener dep)   ├─→ T6 (AboutSection component) → T7 (wire into SettingsWindow) → T8 (verify)
T3 (snk-updater Rust state)    ─→ T4 (TS bindings) ─┘
T5 (delete @snk/ocr)           ───────────────────────────────────────────────────────────────────┘
```

T1, T2, T3, T5 independent → parallel. T4 needs T3. T6 needs T1, T2, T4. T7 needs T6. T8 last.

---

## Task 1: Vite — inject `__GIT_SHA__` and `__UPDATER_FINGERPRINT__` at build time

**Files:**
- Modify: `app/vite.config.ts`
- Create: `app/src/env.d.ts` (TS ambient declarations for the new globals)

**Context:** The About section displays the build's git short-SHA and the bundled updater pubkey's minisign key ID ("fingerprint"). Both are knowable at build time from the working tree + `app/src-tauri/tauri.conf.json`. Computing them in Vite avoids new Rust commands and ensures the displayed values reflect the actual shipped build.

**Step 1: Modify `app/vite.config.ts`**

Replace the file contents with:

```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

function gitShortSha(): string {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf8' }).trim();
  } catch {
    return 'dev';
  }
}

function updaterFingerprint(): string {
  try {
    const cfgRaw = readFileSync(
      resolve(__dirname, 'src-tauri/tauri.conf.json'),
      'utf8',
    );
    const cfg = JSON.parse(cfgRaw) as {
      plugins?: { updater?: { pubkey?: string } };
    };
    const pubkeyB64 = cfg.plugins?.updater?.pubkey ?? '';
    // pubkey is base64-encoded minisign public key text. First line is the
    // comment ("untrusted comment: minisign public key: <KEYID>"). Pull
    // the KEYID out of the comment — that's the displayed fingerprint.
    const decoded = Buffer.from(pubkeyB64, 'base64').toString('utf8');
    const match = /minisign public key:\s*([0-9A-Fa-f]+)/i.exec(decoded);
    return match?.[1] ?? 'unknown';
  } catch {
    return 'unknown';
  }
}

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  define: {
    __GIT_SHA__: JSON.stringify(gitShortSha()),
    __UPDATER_FINGERPRINT__: JSON.stringify(updaterFingerprint()),
  },
  build: {
    target: 'es2022',
    sourcemap: true,
  },
});
```

**Step 2: Create `app/src/env.d.ts`**

```ts
declare const __GIT_SHA__: string;
declare const __UPDATER_FINGERPRINT__: string;
```

**Step 3: Verify typecheck + build pick up the new globals**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app build
```

Expected: both clean. The build output should not error on the new `define` keys.

**Step 4: Commit**

```bash
git add app/vite.config.ts app/src/env.d.ts
git commit -m "feat(ui): inject git SHA + updater fingerprint at build time"
```

---

## Task 2: Add `tauri-plugin-opener` dependency + permissions

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/src/main.rs` (init the plugin)
- Modify: `app/package.json` (add `@tauri-apps/plugin-opener`)
- Modify: `app/src-tauri/capabilities/default.json` (add opener permissions)

**Context:** The About section needs to open filesystem paths (Data dir, Log dir) and URLs (Privacy link, License link). `tauri-plugin-opener` is the Tauri 2 canonical plugin for both, replacing the older `tauri-plugin-shell::open`. The TS side exposes `openPath()` and `openUrl()` from `@tauri-apps/plugin-opener`.

**Step 1: Add Rust dependency**

In `app/src-tauri/Cargo.toml`, find the `[dependencies]` section and add:

```toml
tauri-plugin-opener = "2"
```

If a `[workspace.dependencies]` entry exists in the root `Cargo.toml`, prefer adding it there and using `tauri-plugin-opener.workspace = true` in `app/src-tauri/Cargo.toml`. Check first:

```bash
grep tauri-plugin Cargo.toml | head -5
```

If you see workspace pins for the other tauri-plugins, follow that pattern. Otherwise, inline `tauri-plugin-opener = "2"` is fine.

**Step 2: Initialize the plugin**

In `app/src-tauri/src/main.rs`, find the `tauri::Builder::default()` chain (or wherever the existing plugins are initialized — `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` is the pattern). Add:

```rust
.plugin(tauri_plugin_opener::init())
```

next to the other `.plugin(...)` calls. The exact line number depends on the current file; place it after the other tauri-plugin inits and before the snk-* plugin inits.

**Step 3: Add TS dependency**

In `app/package.json`'s `dependencies`, add:

```json
"@tauri-apps/plugin-opener": "^2.0.0"
```

(alphabetically sorted between `@tauri-apps/plugin-global-shortcut` and any later `@tauri-apps/` entry, or in any order — pnpm preserves whatever you write).

Run:

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm install
```

**Step 4: Add opener permissions**

In `app/src-tauri/capabilities/default.json`, append to the `permissions` array (before the closing `]`):

```json
"opener:default",
"opener:allow-open-path",
"opener:allow-open-url"
```

The exact permission names follow `tauri-plugin-opener`'s convention — if `cargo check -p snapper-keeper-app` errors with a permission-not-found message, replace the name with whatever the plugin actually defines (check `app/src-tauri/gen/` after a build, or the plugin's docs).

**Step 5: Verify**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
cargo check -p snapper-keeper-app
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 6: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/src/main.rs app/package.json app/src-tauri/capabilities/default.json pnpm-lock.yaml Cargo.lock
git commit -m "chore(deps): add tauri-plugin-opener for path + url opening"
```

(Stage `pnpm-lock.yaml` and `Cargo.lock` only if they actually changed — `git status` will show them. Stage `Cargo.toml` only at the root if you used the workspace pattern.)

---

## Task 3: snk-updater Rust — add `last_check_at` + `restart_app` command

**Files:**
- Modify: `crates/snk-updater/src/plugin.rs`

**Context:** `UpdaterState` today holds only `status`. About panel needs to display the last check timestamp. Also needs a way to trigger app restart after an update reaches `Ready`. Both are small additions to the existing plugin.

**Step 1: Write the failing test**

Append to `crates/snk-updater/src/plugin.rs` (or `crates/snk-updater/tests/` if you prefer integration tests — the existing pattern in the file is inline `#[cfg(test)] mod tests {}`; if there's no existing test module, add one). Test the state's last_check_at getter/setter logic in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_state_last_check_at_starts_none() {
        let state = UpdaterState::new();
        assert!(state.get_last_check_at().is_none());
    }

    #[test]
    fn updater_state_records_last_check_at() {
        let state = UpdaterState::new();
        state.set_last_check_at(1716662400000);
        assert_eq!(state.get_last_check_at(), Some(1716662400000));
    }
}
```

**Step 2: Run test, verify it fails**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
cargo test -p snk-updater
```

Expected: 2 failing tests with `no method named 'get_last_check_at'` / `set_last_check_at`.

**Step 3: Implement**

In `crates/snk-updater/src/plugin.rs`:

1. Add `last_check_at: Mutex<Option<i64>>` to `UpdaterState`:

```rust
pub struct UpdaterState {
    status: Mutex<UpdateStatus>,
    last_check_at: Mutex<Option<i64>>,
}

impl UpdaterState {
    fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
            last_check_at: Mutex::new(None),
        }
    }

    fn set_status(&self, s: UpdateStatus) {
        if let Ok(mut lock) = self.status.lock() {
            *lock = s;
        }
    }

    fn get_status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UpdateStatus::Idle)
    }

    fn set_last_check_at(&self, ts: i64) {
        if let Ok(mut lock) = self.last_check_at.lock() {
            *lock = Some(ts);
        }
    }

    fn get_last_check_at(&self) -> Option<i64> {
        self.last_check_at.lock().ok().and_then(|l| *l)
    }
}
```

2. In `do_update_check`, record the timestamp at the start:

```rust
async fn do_update_check<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    let state = app.state::<UpdaterState>();
    state.set_status(UpdateStatus::Checking);
    state.set_last_check_at(chrono::Utc::now().timestamp_millis());
    let _ = app.emit("updater:status-changed", UpdateStatus::Checking);
    // ... rest unchanged
```

(If `chrono` isn't already a dep of `snk-updater`, add `chrono.workspace = true` to `crates/snk-updater/Cargo.toml` — it should be inheritable since other crates use it.)

3. Add two new commands at the bottom of the existing command list:

```rust
#[tauri::command]
pub fn get_last_check_at<R: Runtime>(app: AppHandle<R>) -> Option<i64> {
    app.state::<UpdaterState>().get_last_check_at()
}

#[tauri::command]
pub fn restart_app<R: Runtime>(app: AppHandle<R>) {
    app.restart();
}
```

4. Register them in the `generate_handler!` macro:

```rust
.invoke_handler(tauri::generate_handler![
    check_for_update,
    get_update_status,
    get_last_check_at,
    restart_app
])
```

**Step 4: Run tests, verify pass**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
cargo test -p snk-updater
```

Expected: existing tests + the 2 new tests all passing.

**Step 5: Verify clippy + fmt**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
cargo fmt -p snk-updater -- --check
cargo clippy -p snk-updater -- -D warnings
```

Expected: both clean.

**Step 6: Commit**

```bash
git add crates/snk-updater/src/plugin.rs crates/snk-updater/Cargo.toml
git commit -m "feat(updater): expose last check timestamp + restart_app command"
```

(Stage `Cargo.toml` only if you added `chrono.workspace = true`.)

---

## Task 4: `@snk/updater` TS bindings — add `lastCheckedAt()` + `restart()`

**Files:**
- Modify: `packages/snk-updater/src/index.ts`
- Modify: `packages/snk-updater/src/index.test.ts`

**Step 1: Write the failing test**

Append to `packages/snk-updater/src/index.test.ts`:

```ts
import { lastCheckedAt, restart } from './index';

describe('@snk/updater extended bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('lastCheckedAt returns the epoch-ms or null', async () => {
    mockedInvoke.mockResolvedValue(1716662400000);
    expect(await lastCheckedAt()).toBe(1716662400000);
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-updater|get_last_check_at');

    mockedInvoke.mockResolvedValue(null);
    expect(await lastCheckedAt()).toBeNull();
  });

  it('restart invokes plugin:snk-updater|restart_app', async () => {
    await restart();
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-updater|restart_app');
  });
});
```

**Step 2: Run test, verify fail**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/updater test
```

Expected: 2 new failing tests (`Cannot find export 'lastCheckedAt'` / `'restart'`).

**Step 3: Extend bindings**

Replace `packages/snk-updater/src/index.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';

import type { UpdateStatus } from './types';

export * from './types';

export function checkForUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|check_for_update');
}

export function getUpdateStatus(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|get_update_status');
}

export function lastCheckedAt(): Promise<number | null> {
  return invoke<number | null>('plugin:snk-updater|get_last_check_at');
}

export function restart(): Promise<void> {
  return invoke<void>('plugin:snk-updater|restart_app');
}
```

**Step 4: Run tests, verify pass**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/updater test
```

Expected: all 4 tests passing (2 original + 2 new).

**Step 5: Commit**

```bash
git add packages/snk-updater/src/index.ts packages/snk-updater/src/index.test.ts
git commit -m "feat(updater): TS bindings for lastCheckedAt + restart"
```

---

## Task 5: Delete `@snk/ocr` package

**Files:**
- Delete: entire `packages/snk-ocr/` directory
- Modify: `app/vitest.config.ts` (drop the `@snk/ocr` alias on lines 41-42)

**Context:** `@snk/ocr` exports only `ocrStatus()` returning a hardcoded `"running"` from the snk-ocr Rust plugin's stub `ocr_status` command. About panel doesn't use it (per the scope decision: no OCR section). No other code imports it (verified — only docs and the package's own test reference it). Resolves #62 by deletion.

The snk-ocr Rust plugin and its real OCR pipeline stay — only the dead TS binding goes.

**Step 1: Verify no other consumers**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
grep -r "@snk/ocr" app/ packages/ crates/ --include="*.ts" --include="*.tsx" --include="*.rs" --include="*.json" --include="*.yaml" 2>&1 | grep -v packages/snk-ocr | grep -v node_modules
```

Expected output: only `app/vitest.config.ts` (the alias to remove).

If anything else turns up (a real import in source code), STOP and message team-lead — scope changed.

**Step 2: Delete the package**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
git rm -r packages/snk-ocr
```

**Step 3: Remove the alias**

In `app/vitest.config.ts`, find and delete this line inside the `resolve.alias` block:

```ts
      '@snk/ocr': new URL('../packages/snk-ocr/src/index.ts', import.meta.url).pathname,
```

The line above (`'@snk/annotate'`) and below (`'@snk/updater'`) stay.

**Step 4: Drop the snk-ocr permission from capabilities**

In `app/src-tauri/capabilities/default.json`, the `permissions` array currently includes `"snk-ocr:default"`. Leave it — the snk-ocr Rust plugin is still loaded and emits events; the permission is needed for any future TS consumers (e.g., a future About section if we change our minds). Deleting just the TS binding does not require dropping the IPC permission.

**Step 5: Refresh pnpm install to drop the workspace entry**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm install
```

Expected: pnpm-lock.yaml updates to remove the @snk/ocr workspace package.

**Step 6: Verify the suite still passes**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm -r --filter "@snk/*" --filter @snk/app test
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Expected: all clean. Test count drops by 2 (the @snk/ocr binding's 2 tests are gone).

**Step 7: Commit**

```bash
git add app/vitest.config.ts pnpm-lock.yaml
# `git rm -r` already staged the deletion in step 2.
git diff --cached --stat
git commit -m "chore: delete unused @snk/ocr TS package (closes #62)"
```

`git diff --cached --stat` should show the package directory deleted + vitest.config.ts modified + pnpm-lock.yaml updated. Nothing else.

---

## Task 6: `AboutSection.tsx` — new component

**Files:**
- Create: `app/src/windows/settings/AboutSection.tsx`
- Create: `app/src/windows/settings/AboutSection.test.tsx`

**Context:** New React component. Uses shared primitives from PR A (`SettingsSection`, `SettingRow`, `Button`, `useModal`). Renders 8 rows in this order:

1. Version: `getVersion()` + ` (` + `__GIT_SHA__` + `)`
2. Data directory: path + Open button
3. Log directory: path + Open button
4. Updater pubkey fingerprint: `__UPDATER_FINGERPRINT__`
5. Updater last check: relative time ("2m ago", "never") + tooltip with absolute ISO
6. Updater status: from `getUpdateStatus()` ("Up to date", "Update available v1.2.3", "Checking…", "Downloading 47%", "Ready to install v1.2.3", "Error: …") + Check Now button (disabled while checking/downloading)
7. Privacy link
8. License link

When the user clicks Check Now → `checkForUpdate()` → component subscribes to `updater:status-changed` events and re-renders. When status becomes `ready`, `useModal().confirm` opens "Update v1.2.3 is ready. Restart now to install?" with primary action `restart()`.

**Step 1: Write the failing test**

Create `app/src/windows/settings/AboutSection.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';

import { ModalProvider } from '../../components/Modal';
import { AboutSection } from './AboutSection';
import { renderWithQuery } from '../../test/renderWithQuery';

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.1.2'),
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn().mockResolvedValue('/mock/data'),
  appLogDir: vi.fn().mockResolvedValue('/mock/log'),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn().mockResolvedValue(undefined),
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedListen.mockReset().mockResolvedValue(() => {});
  vi.mocked(getVersion).mockClear();

  // Add a modal-root div for ModalProvider.
  const existing = document.getElementById('modal-root');
  if (existing) existing.remove();
  const root = document.createElement('div');
  root.id = 'modal-root';
  document.body.appendChild(root);
});

function setStatusResponses(opts: {
  lastCheckedAt?: number | null;
  status?: { kind: string; [k: string]: unknown };
} = {}) {
  mockedInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'plugin:snk-updater|get_update_status') {
      return Promise.resolve(opts.status ?? { kind: 'idle' });
    }
    if (cmd === 'plugin:snk-updater|get_last_check_at') {
      return Promise.resolve(opts.lastCheckedAt ?? null);
    }
    if (cmd === 'plugin:snk-updater|check_for_update') {
      return Promise.resolve(opts.status ?? { kind: 'idle' });
    }
    if (cmd === 'plugin:snk-updater|restart_app') {
      return Promise.resolve(undefined);
    }
    return Promise.resolve(null);
  });
}

function renderAbout() {
  return renderWithQuery(
    <ModalProvider>
      <AboutSection />
    </ModalProvider>,
  );
}

describe('<AboutSection />', () => {
  it('renders the section header', async () => {
    setStatusResponses();
    renderAbout();
    expect(await screen.findByRole('heading', { name: 'About', level: 2 })).toBeInTheDocument();
  });

  it('renders the app version with git sha', async () => {
    setStatusResponses();
    renderAbout();
    // __GIT_SHA__ is injected by Vite at build time; in tests it'll be
    // whatever the test runner provides (test setup may stub or it falls
    // through to the build's value). Match the version + parenthesized sha.
    await waitFor(() => {
      expect(screen.getByText(/0\.1\.2 \(.+\)/)).toBeInTheDocument();
    });
  });

  it('renders the data dir and log dir paths', async () => {
    setStatusResponses();
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText('/mock/data')).toBeInTheDocument();
      expect(screen.getByText('/mock/log')).toBeInTheDocument();
    });
  });

  it('renders the updater fingerprint', async () => {
    setStatusResponses();
    renderAbout();
    // __UPDATER_FINGERPRINT__ is injected by Vite; check for any
    // non-"unknown" value, or for "unknown" if the build couldn't parse.
    await waitFor(() => {
      const row = screen.getByText(/Fingerprint/i).closest('div.flex')!;
      const val = row.textContent ?? '';
      expect(val.length).toBeGreaterThan('Fingerprint'.length);
    });
  });

  it('renders "never" for last check when null', async () => {
    setStatusResponses({ lastCheckedAt: null });
    renderAbout();
    await waitFor(() => {
      const row = screen.getByText(/Last check/i).closest('div.flex')!;
      expect(row.textContent).toMatch(/never/i);
    });
  });

  it('renders the updater status text for idle', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText(/Up to date/i)).toBeInTheDocument();
    });
  });

  it('renders Check Now button which is enabled when idle', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    const btn = await screen.findByRole('button', { name: /Check Now/i });
    expect(btn).toBeEnabled();
  });

  it('Check Now button is disabled while status is checking', async () => {
    setStatusResponses({ status: { kind: 'checking' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Check/i })).toBeDisabled();
    });
  });

  it('clicking Check Now calls check_for_update', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    const btn = await screen.findByRole('button', { name: /Check Now/i });
    fireEvent.click(btn);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-updater|check_for_update');
    });
  });

  it('shows restart modal when status reaches "ready"', async () => {
    // Initial status is ready (component should immediately surface modal).
    setStatusResponses({ status: { kind: 'ready', version: '1.2.3' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText(/Restart/i)).toBeInTheDocument();
    });
  });

  it('clicking Privacy link calls openUrl with the privacy URL', async () => {
    setStatusResponses();
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    const link = await screen.findByRole('button', { name: /Privacy/i });
    fireEvent.click(link);
    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(expect.stringContaining('github.com'));
    });
  });

  it('clicking License link calls openUrl with the license URL', async () => {
    setStatusResponses();
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    const link = await screen.findByRole('button', { name: /License/i });
    fireEvent.click(link);
    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(expect.stringContaining('github.com'));
    });
  });

  it('clicking Open on a path row calls openPath', async () => {
    setStatusResponses();
    const { openPath } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    // Two Open buttons (data + log). Both should be openPath.
    const openButtons = await screen.findAllByRole('button', { name: /Open/i });
    expect(openButtons.length).toBeGreaterThanOrEqual(2);
    fireEvent.click(openButtons[0]!);
    await waitFor(() => {
      expect(openPath).toHaveBeenCalled();
    });
  });
});
```

**Step 2: Run test, verify fail**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app test -- --run src/windows/settings/AboutSection.test.tsx
```

Expected: FAIL with `Cannot find module './AboutSection'`.

**Step 3: Implement `AboutSection.tsx`**

Create `app/src/windows/settings/AboutSection.tsx`:

```tsx
import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getVersion } from '@tauri-apps/api/app';
import { appDataDir, appLogDir } from '@tauri-apps/api/path';
import { listen } from '@tauri-apps/api/event';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';

import {
  checkForUpdate,
  getUpdateStatus,
  lastCheckedAt,
  restart,
  type UpdateStatus,
} from '@snk/updater';

import { SettingsSection } from '../../components/SettingsSection';
import { SettingRow } from '../../components/SettingRow';
import { Button } from '../../components/Button';
import { useModal } from '../../components/Modal';

const PRIVACY_URL = 'https://github.com/ehartye/snapper-keeper/blob/main/PRIVACY.md';
const LICENSE_URL = 'https://github.com/ehartye/snapper-keeper/blob/main/LICENSE';

function formatStatus(s: UpdateStatus): string {
  switch (s.kind) {
    case 'idle':
      return 'Up to date';
    case 'checking':
      return 'Checking…';
    case 'available':
      return `Update available: v${s.version}`;
    case 'downloading':
      return `Downloading ${Math.round(s.percent)}%`;
    case 'ready':
      return `Ready to install v${s.version}`;
    case 'error':
      return `Error: ${s.detail}`;
  }
}

function formatRelative(ts: number | null): string {
  if (ts === null) return 'never';
  const diffMs = Date.now() - ts;
  if (diffMs < 60_000) return 'just now';
  if (diffMs < 3_600_000) return `${Math.floor(diffMs / 60_000)}m ago`;
  if (diffMs < 86_400_000) return `${Math.floor(diffMs / 3_600_000)}h ago`;
  return `${Math.floor(diffMs / 86_400_000)}d ago`;
}

export function AboutSection() {
  const modal = useModal();

  const versionQ = useQuery({
    queryKey: ['app-version'],
    queryFn: () => getVersion(),
  });
  const dataDirQ = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => appDataDir(),
  });
  const logDirQ = useQuery({
    queryKey: ['app-log-dir'],
    queryFn: () => appLogDir(),
  });

  // Updater status — initial value comes from the Rust state, then
  // updater:status-changed events update live.
  const [status, setStatus] = useState<UpdateStatus>({ kind: 'idle' });
  const [lastCheck, setLastCheck] = useState<number | null>(null);
  const [restartPrompted, setRestartPrompted] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getUpdateStatus().then((s) => {
      if (!cancelled) setStatus(s);
    });
    void lastCheckedAt().then((ts) => {
      if (!cancelled) setLastCheck(ts);
    });
    const unlistenPromise = listen<UpdateStatus>('updater:status-changed', (e) => {
      setStatus(e.payload);
      if (e.payload.kind === 'checking' || e.payload.kind === 'idle') {
        void lastCheckedAt().then((ts) => setLastCheck(ts));
      }
    });
    return () => {
      cancelled = true;
      void unlistenPromise.then((fn) => fn());
    };
  }, []);

  // When status reaches `ready`, prompt the user once to restart.
  useEffect(() => {
    if (status.kind === 'ready' && !restartPrompted) {
      setRestartPrompted(true);
      modal.confirm({
        title: 'Update ready',
        body: `Update v${status.version} is ready. Restart now to install?`,
        confirmLabel: 'Restart',
        cancelLabel: 'Later',
        onConfirm: () => restart(),
      });
    }
  }, [status, restartPrompted, modal]);

  const isChecking = status.kind === 'checking' || status.kind === 'downloading';
  const sha = __GIT_SHA__;
  const fingerprint = __UPDATER_FINGERPRINT__;

  return (
    <SettingsSection title="About">
      <SettingRow label="Version">
        <span className="text-sm text-fg-muted font-mono">
          {versionQ.data ? `${versionQ.data} (${sha})` : `… (${sha})`}
        </span>
      </SettingRow>
      <SettingRow label="Data directory" description={dataDirQ.data ?? ''}>
        <Button
          variant="secondary"
          onClick={() => dataDirQ.data && void openPath(dataDirQ.data)}
          disabled={!dataDirQ.data}
        >
          Open
        </Button>
      </SettingRow>
      <SettingRow label="Log directory" description={logDirQ.data ?? ''}>
        <Button
          variant="secondary"
          onClick={() => logDirQ.data && void openPath(logDirQ.data)}
          disabled={!logDirQ.data}
        >
          Open
        </Button>
      </SettingRow>
      <SettingRow
        label="Fingerprint"
        description="Updater public key identifier (verify against release notes)"
      >
        <span className="text-xs text-fg-muted font-mono">{fingerprint}</span>
      </SettingRow>
      <SettingRow label="Last check">
        <span
          className="text-sm text-fg-muted"
          title={lastCheck ? new Date(lastCheck).toISOString() : ''}
        >
          {formatRelative(lastCheck)}
        </span>
      </SettingRow>
      <SettingRow label="Status" description={formatStatus(status)}>
        <Button
          onClick={() => void checkForUpdate()}
          disabled={isChecking}
        >
          Check Now
        </Button>
      </SettingRow>
      <SettingRow label="Privacy">
        <Button variant="secondary" onClick={() => void openUrl(PRIVACY_URL)}>
          Privacy
        </Button>
      </SettingRow>
      <SettingRow label="License">
        <Button variant="secondary" onClick={() => void openUrl(LICENSE_URL)}>
          License
        </Button>
      </SettingRow>
    </SettingsSection>
  );
}
```

**Step 4: Run test, verify pass**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app test -- --run src/windows/settings/AboutSection.test.tsx
```

Expected: all 13 tests passing.

**Step 5: Run lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 6: Commit**

```bash
git add app/src/windows/settings/AboutSection.tsx app/src/windows/settings/AboutSection.test.tsx
git commit -m "feat(ui): add About section with version, paths, updater status (closes #36)"
```

---

## Task 7: Wire `<AboutSection />` into `SettingsWindow.tsx`

**Files:**
- Modify: `app/src/windows/settings/SettingsWindow.tsx`

**Step 1: Modify `SettingsWindow.tsx`**

Add the import near the other component imports:

```tsx
import { AboutSection } from './AboutSection';
```

Append `<AboutSection />` as the last section, after the existing Startup section. The final JSX inside the scrollable content div:

```tsx
        <SettingsSection title="Startup">
          <SettingRow
            label="Launch at login"
            description="Start snapper-keeper automatically when you sign in"
          >
            <Toggle value={autostart} onChange={setAutostart} />
          </SettingRow>
        </SettingsSection>

        <AboutSection />
      </div>
    </main>
```

**Step 2: Run the existing SettingsWindow tests to confirm no regression**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app test -- --run src/windows/settings/SettingsWindow.test.tsx
```

Expected: all 5 tests passing. (The existing tests don't assert on AboutSection internals — they just verify the headers render.)

**Step 3: Add one new SettingsWindow test for the About section header**

In `app/src/windows/settings/SettingsWindow.test.tsx`, find the existing test:

```tsx
it('renders the Settings header and Appearance + Capture + Clipboard + OCR sections', async () => {
```

Add "About" to the expected sections list:

```tsx
  it('renders Settings header + Appearance + Capture + Clipboard + OCR + About sections', async () => {
    renderWithQuery(<SettingsWindow />);
    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('Capture')).toBeInTheDocument();
    expect(screen.getByText('Clipboard')).toBeInTheDocument();
    expect(screen.getByText('OCR')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'About', level: 2 })).toBeInTheDocument();
  });
```

Run:

```bash
pnpm --filter @snk/app test -- --run src/windows/settings/SettingsWindow.test.tsx
```

Expected: 5 tests passing (the updated one still counts as 1).

**Step 4: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 5: Commit**

```bash
git add app/src/windows/settings/SettingsWindow.tsx app/src/windows/settings/SettingsWindow.test.tsx
git commit -m "feat(ui): mount AboutSection in SettingsWindow"
```

---

## Task 8: Final verification — full suite + lint + typecheck + build + cargo

**Files:** None modified. Verification only.

**Step 1: Full TS suite**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm -r --filter "@snk/*" --filter @snk/app test
```

Expected: ~225 tests passing (212 baseline + 13 AboutSection - 2 deleted @snk/ocr + 2 new @snk/updater + 1 modified SettingsWindow = ~226). Report exact count.

**Step 2: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 3: Vite build**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
pnpm --filter @snk/app build
```

Expected: succeeds. The built bundle should embed real `__GIT_SHA__` and `__UPDATER_FINGERPRINT__` values (not "dev" / "unknown" unless git/config are absent).

**Step 4: Cargo workspace check**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-about-panel
cargo fmt -- --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
cargo test --workspace --exclude snapper-keeper-app
```

Expected: all three clean. snk-updater's new commands and `last_check_at` state compile + lint + test.

**Step 5: Hand off**

Push branch + open PR titled `feat(ui): About panel + clean up @snk/ocr (closes #36, #62)`.

---

## Self-review notes

1. **Spec coverage:** Version + git SHA (T1, T6 row 1), data dir + Open (T2 + T6 row 2), log dir + Open (T2 + T6 row 3), pubkey fingerprint (T1 + T6 row 4), last check + status (T3 + T4 + T6 rows 5+6), Privacy + License links (T2 + T6 rows 7+8). #62: @snk/ocr deleted (T5).
2. **Placeholders:** none — every step has concrete code/commands.
3. **Naming consistency:** `UpdateStatus` discriminator is `kind` across Rust + TS (verified via existing snk-updater code). `lastCheckedAt`, `restart` match between TS bindings (T4) and Rust commands (T3). `AboutSection` is the component name everywhere (T6 + T7).
4. **Buildability:** T1+T2+T3+T5 parallel; T4 needs T3; T6 needs T1+T2+T4; T7 needs T6; T8 last. All commands shown explicitly.

## Plan-as-source-of-truth reminder

If any implementer finds a real bug in the plan, raise to team-lead BEFORE applying. Per memory `[[feedback_plan_as_source_of_truth]]`.
