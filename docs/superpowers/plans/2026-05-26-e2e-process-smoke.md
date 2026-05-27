# E2E process-smoke implementation plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Replace PR #144's failing WebDriver smoke with two layers: per-plugin `tauri::test::mock_builder` ACL smoke (Layer 1) and cross-OS packaged-binary process-smoke (Layer 2). See `docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md`.

**Architecture:** Two layers. Layer 1 runs in the existing `rust-test` job — each plugin gets a `tests/command_acl_smoke.rs` that builds the plugin via `tauri::test::mock_builder` and (best-effort) invokes its commands. Layer 2 is a new `e2e-process-smoke` matrix job (`windows-latest` + `macos-latest`) that builds the packaged binary, launches it, scrapes stdout/stderr/app-log for known-bad signatures, captures a native screenshot, terminates, and uploads artifacts.

**Tech Stack:** Rust 2021 + Tauri 2 (`tauri::test::MockRuntime`), Node 20 (Layer 2 harness, no extra npm deps — uses Node built-ins), PowerShell + Bash (CI), GitHub Actions matrix.

**Branch:** `copilot/add-e2e-smoke-tests-per-os` (PR #144 — rewriting in-place; the existing WebDriver code is deleted in Task 2).

**Issue:** [#47](https://github.com/ehartye/snapper-keeper/issues/47)

---

## Plugin inventory (Layer 1 scope)

Confirmed by grep of `pub fn init<R: Runtime>() -> TauriPlugin<R>` + `tauri::generate_handler!`:

| Crate | Has command surface | Notes |
|---|---|---|
| `snk-annotate` | yes | commands in `src/commands.rs`, init in `src/plugin.rs:4` |
| `snk-capture` | yes | commands in `src/commands.rs`, init in `src/plugin.rs:4` |
| `snk-clipboard` | yes | commands in `src/commands.rs`, init in `src/plugin.rs:6` |
| `snk-hotkeys` | yes | init in `src/lib.rs:55` (no separate plugin.rs) |
| `snk-library` | yes | commands in `src/commands.rs`, init in `src/plugin.rs:14` (19 commands) |
| `snk-ocr` | yes | init in `src/plugin.rs:72` (2 commands: `ocr_status`, `get_ocr_words`) |
| `snk-updater` | yes | init in `src/plugin.rs:233` |
| `snk-pii` | **no** | pure-data crate, empty `[dependencies]`. Skipped. |

7 plugins get a Layer 1 test.

---

### Task 1: Pre-flight — prove plugin-init smoke pattern for one plugin

This task de-risks the spec's Risk 1 ("Tauri 2 `tauri::test` API surface stability") on a single plugin (`snk-library`) before fanning out to all 7.

**Implementation discovery (2026-05-26):** During pre-flight, the full `mock_builder().plugin(...).build(mock_context(noop_assets()))` chain produced `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139) on the implementer's Windows + OneDrive environment — `tauri::Wry`/`MockRuntime` monomorphization pulls in a windowing-system DLL chain (comctl32, ole32, gdi32, dwmapi, shell32, user32) that fails to load before the test entrypoint runs. The function-pointer-only variant works cleanly. Plan revised to use that variant — matches the spec's Risk 1 mitigation ("fallback is per-plugin command unit tests without a full mock app — lower fidelity, still catches the wiring"). The deeper ACL gotcha coverage moves to Layer 2 (the real packaged binary exercises all plugins' ACL at startup).

**Files:**
- Create: `crates/snk-library/tests/command_acl_smoke.rs`

(No `Cargo.toml` change needed — the function-pointer variant uses only `tauri` symbols already in `[dependencies]`.)

**Step 1: Write the test**

Create `crates/snk-library/tests/command_acl_smoke.rs`:

```rust
//! Layer 1 plugin-init smoke for snk-library.
//!
//! Asserts that `snk_library::init` exists with the expected signature
//! and monomorphizes for `tauri::Wry`. Catches:
//!   * accidental removal/rename of `init`
//!   * accidental signature drift away from `fn() -> TauriPlugin<R>`
//!   * compile errors anywhere in the plugin's code path
//!
//! Does NOT catch the deeper "three coordinated entries" ACL gotcha
//! (CLAUDE.md line 48) — the cheap way to catch that (mock_builder +
//! build + invoke) requires monomorphizing the runtime, which pulls in
//! a Windows DLL chain that fails to load in some local environments
//! (see plan task 1's "Implementation discovery"). Layer 2's real
//! packaged-binary smoke covers that surface.
//!
//! Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

#[test]
fn init_symbol_exists() {
    let _: fn() -> tauri::plugin::TauriPlugin<tauri::Wry> = snk_library::init;
}
```

**Step 2: Run test**

```bash
cargo test --package snk-library --test command_acl_smoke
```

Expected: **PASSES**. The test is a compile-time API-surface assertion plus a no-op runtime entrypoint, so it passes trivially on a healthy main and fails to compile only when `init` is renamed/removed or its signature changes.

**Step 3: Commit**

```bash
git add crates/snk-library/tests/command_acl_smoke.rs
git commit -m "test(snk-library): add Layer 1 plugin-init smoke

Asserts snk_library::init exists with the expected fn() -> TauriPlugin<R>
signature. Compile-time API-surface check; deeper ACL gotcha coverage
moves to Layer 2 — the full mock_builder().build() chain triggered a
Windows DLL load failure (STATUS_ENTRYPOINT_NOT_FOUND) in the
implementer's environment, and per the spec's Risk 1 mitigation we
fall back to the cheaper variant rather than fight the DLL chain.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 2: Remove PR #144's WebDriver pieces

Pure deletions — clears the slate before adding the new approach.

**Files:**
- Delete: `e2e/windows-smoke.mjs`
- Modify: `.github/workflows/ci.yml` (remove `e2e-windows-smoke` job)
- Modify: `package.json` (remove `test:e2e:windows` script)
- Modify: `README.md` (remove "Windows runtime smoke" CI summary section)

**Step 1: Delete the WebDriver smoke script**

```bash
git rm e2e/windows-smoke.mjs
```

**Step 2: Remove the `e2e-windows-smoke` job from CI**

Open `.github/workflows/ci.yml`. Delete the entire `e2e-windows-smoke:` job block (currently lines 178–259, but verify line numbers — file may have shifted). The block starts with the comment `# Intentional scope:` and ends after the `Upload Windows runtime artifact` step's `if-no-files-found: error` line.

Leave all other jobs (`verify-docs`, `verify-theme-keys`, `verify-csp`, `lint-typecheck`, `ts-test`, `rust-test`, `coverage`, `build-app`) untouched.

**Step 3: Remove `test:e2e:windows` script**

Open `package.json`. Find the `"test:e2e:windows"` entry in the `scripts` block and delete it (plus the trailing comma on the previous line or the entry itself, whichever keeps the JSON valid).

Run `node -e "JSON.parse(require('fs').readFileSync('package.json','utf8'))"` to verify the file still parses.

**Step 4: Remove README WebDriver smoke reference**

Open `README.md`. Find the section describing the Windows WebDriver smoke job (search for "tauri-driver" or "WebDriver"). Delete the paragraph(s). Don't remove the entire CI section heading — leave it for Task 16 to update with the new description.

**Step 5: Verify CI workflow YAML is still valid**

```bash
# Optional but recommended — use any local yamllint or just attempt a dry parse
node -e "const y=require('js-yaml'); y.load(require('fs').readFileSync('.github/workflows/ci.yml','utf8'))" || echo "no js-yaml available; skip"
```

Verify by eyeballing that the `jobs:` keys still indent cleanly.

**Step 6: Commit**

```bash
git add .github/workflows/ci.yml package.json README.md e2e/windows-smoke.mjs
git commit -m "ci: remove tauri-driver WebDriver smoke — superseded by process-smoke

PR #144's WebDriver-based smoke (tauri-driver + msedgedriver pinned to
runner Edge build) failed 5 consecutive CI rounds in the Edge-driver-pin
step; the smoke script itself never executed. Structural flake from
runner-Edge / WebView2 / msedgedriver version drift.

Replaced by the two-layer approach in the companion design doc. Layer 1
lands in subsequent commits; Layer 2 follows.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 3: Add an explicit `app_ready` smoke marker in `main.rs`

Layer 2's scraper waits for a known log line to confirm steady-state. Today's closest signal is `logging::init`'s `"logging initialized"` log, which is coupled to a comment inside `main.rs`. Add an explicit, smoke-targeted marker so Layer 2 doesn't depend on incidental log lines.

**Files:**
- Modify: `app/src-tauri/src/main.rs` (emit one new `tracing::info!` after all plugins + tray + windows are set up)

**Step 1: Find where bootstrap finishes**

Read `app/src-tauri/src/main.rs`. Find the point where the Tauri app finishes its `.setup(...)` closure successfully — i.e., after all plugins are registered, the tray icon is built, and the main windows are created. This is typically near the end of `fn main()`'s `Builder::default().setup(...).run(...)` chain or inside the `.setup(...)` closure's `Ok(())` return.

If the bootstrap is split across multiple functions, find the last one to run before `run(...)` is called.

**Step 2: Add the marker emit**

Insert immediately before the `.setup(...)` closure's final `Ok(())`, or in the `app.run(...)` event handler's `RunEvent::Ready` arm (Tauri 2 emits this once the app is fully started). Prefer `RunEvent::Ready` because it fires AFTER `setup` and AFTER the tray + window paths complete:

In `main.rs`, find where the app is run. Tauri 2's pattern:

```rust
app.run(|_app_handle, event| {
    match event {
        // ...
        RunEvent::Ready => {
            tracing::info!(target: "snk::smoke", event = "app_ready", "app reached steady state");
        }
        // ...
    }
});
```

If there's no existing `RunEvent::Ready` arm, add one. If the file uses `tauri::Builder::default().run(generate_context!())` without a closure (uncommon in this project), switch to the closure form just to add the `Ready` arm.

The line must include the exact substring `"snk::smoke"` as a tracing target and the literal `event = "app_ready"` field — the Layer 2 scraper looks for the substring `app_ready` near `snk::smoke` to match.

**Step 3: Verify it compiles**

```bash
cargo check -p snapper-keeper-app
```

Expected: compiles cleanly. If `RunEvent` is missing an import, add `use tauri::RunEvent;` near the top of `main.rs`.

**Step 4: Verify the marker emits in dev**

(Optional — only if on an interactive Windows desktop with `pnpm tauri dev` working.) Launch the app and observe stdout. You should see a line like:
```
... INFO snk::smoke: app reached steady state event="app_ready"
```

If you can't run interactively, skip this verification — Layer 2 will exercise it later in CI.

**Step 5: Commit**

```bash
git add app/src-tauri/src/main.rs
git commit -m "feat(app): emit snk::smoke app_ready marker on RunEvent::Ready

Layer 2 process-smoke uses this as the steady-state signal, decoupling
it from incidental logging-init lines. RunEvent::Ready fires after all
plugins, tray, and windows are set up.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Tasks 4–10: Layer 1 plugin-init smoke for the remaining 6 plugins

These mirror Task 1's pattern exactly, varying only the crate name. Each plugin gets its own task / commit (matches CLAUDE.md's "one task = one commit" convention). No `Cargo.toml` changes needed.

**For each of: `snk-annotate`, `snk-capture`, `snk-clipboard`, `snk-hotkeys`, `snk-ocr`, `snk-updater`:**

**Step 1: Create the test file**

Create `crates/<plugin>/tests/command_acl_smoke.rs`:

```rust
//! Layer 1 plugin-init smoke for <plugin>. See snk-library/tests/command_acl_smoke.rs
//! for the design rationale and the discovery that drove this variant.
//!
//! Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

#[test]
fn init_symbol_exists() {
    let _: fn() -> tauri::plugin::TauriPlugin<tauri::Wry> = <plugin_crate>::init;
}
```

Substitute `<plugin_crate>` with the crate's snake_case name as it appears in the workspace `Cargo.toml`:
- `snk-annotate` → `snk_annotate`
- `snk-capture` → `snk_capture`
- `snk-clipboard` → `snk_clipboard`
- `snk-hotkeys` → `snk_hotkeys`
- `snk-ocr` → `snk_ocr`
- `snk-updater` → `snk_updater`

**Step 2: Run test**

```bash
cargo test --package <crate-name> --test command_acl_smoke
```

Expected: PASS. If it fails with a compile error like `cannot find function 'init' in crate`, the plugin's init function is missing or renamed — surface to user.

**Step 3: Commit**

```bash
git add crates/<plugin>/tests/command_acl_smoke.rs
git commit -m "test(<plugin>): add Layer 1 plugin-init smoke

Mirrors snk-library Layer 1 pattern — compile-time assertion that
<plugin_crate>::init exists with the expected signature.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

Repeat per plugin, one commit each.

---

### Task 11: Create the Layer 2 process-smoke harness skeleton

**Files:**
- Create: `e2e/process-smoke.mjs`
- Modify: `package.json` (add `test:e2e:smoke` script)

**Step 1: Write the harness skeleton**

Create `e2e/process-smoke.mjs`:

```javascript
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync, createWriteStream, readFileSync, readdirSync, statSync, copyFileSync } from 'node:fs';
import { access, mkdir, readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { tmpdir, platform } from 'node:os';

const BIN_PATH = process.env.SNK_BINARY_PATH;
const ARTIFACT_DIR = process.env.E2E_ARTIFACT_DIR ?? join(process.cwd(), 'e2e-artifacts');
const APP_DATA_DIR = process.env.SNK_APP_DATA_DIR ?? join(tmpdir(), `snk-smoke-${Date.now()}`);
const READY_TIMEOUT_MS = Number(process.env.SNK_SMOKE_READY_TIMEOUT_MS ?? 20_000);
const STEADY_HOLD_MS = Number(process.env.SNK_SMOKE_HOLD_MS ?? 5_000);
const SHUTDOWN_GRACE_MS = 5_000;

mkdirSync(ARTIFACT_DIR, { recursive: true });
mkdirSync(APP_DATA_DIR, { recursive: true });

const stdoutPath = join(ARTIFACT_DIR, 'process-stdout.log');
const stderrPath = join(ARTIFACT_DIR, 'process-stderr.log');
const resultPath = join(ARTIFACT_DIR, 'result.json');

const checks = [];
let proc;
let failed = false;
let failureReason;

function logCheck(name, ok, detail) {
  checks.push({ name, ok, detail });
}

async function main() {
  if (!BIN_PATH) {
    throw new Error('SNK_BINARY_PATH is required');
  }
  await access(BIN_PATH);
  logCheck('binary exists', true, BIN_PATH);

  const stdoutStream = createWriteStream(stdoutPath, { flags: 'a' });
  const stderrStream = createWriteStream(stderrPath, { flags: 'a' });
  const collected = { stdout: '', stderr: '' };

  proc = spawn(BIN_PATH, [], {
    env: {
      ...process.env,
      SNK_LOG: 'info,snk=debug',
      // App-data dir override — see CLAUDE.md if this env name changes
      XDG_DATA_HOME: APP_DATA_DIR,
      APPDATA: APP_DATA_DIR,
      LOCALAPPDATA: APP_DATA_DIR,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  proc.stdout.on('data', (chunk) => {
    const s = chunk.toString();
    collected.stdout += s;
    stdoutStream.write(s);
  });
  proc.stderr.on('data', (chunk) => {
    const s = chunk.toString();
    collected.stderr += s;
    stderrStream.write(s);
  });

  const ready = await waitForReady(collected, READY_TIMEOUT_MS);
  logCheck('app_ready marker observed', ready.ok, ready.detail);
  if (!ready.ok) throw new Error(`steady-state timeout: ${ready.detail}`);

  // Hold the binary alive so deferred plugin setup / window paint emits anything bad
  await delay(STEADY_HOLD_MS);
  logCheck('steady-state hold completed', true, `${STEADY_HOLD_MS}ms`);

  // Scrape collected output for known-bad signatures (added in next task)
  const scan = scanForKnownBad(collected.stdout + '\n' + collected.stderr);
  for (const finding of scan.findings) {
    logCheck(`scrape: ${finding.category}`, false, finding.evidence);
  }
  if (scan.findings.length > 0) {
    throw new Error(`scrape found ${scan.findings.length} bad-signature match(es)`);
  }
  logCheck('no bad-signature matches', true, `${scan.linesScanned} lines scanned`);

  await terminate(proc);
  logCheck('graceful termination', true, '');
}

async function waitForReady(collected, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (collected.stdout.includes('snk::smoke') && collected.stdout.includes('app_ready')) {
      return { ok: true, detail: 'matched snk::smoke + app_ready in stdout' };
    }
    if (collected.stderr.includes('snk::smoke') && collected.stderr.includes('app_ready')) {
      return { ok: true, detail: 'matched snk::smoke + app_ready in stderr' };
    }
    if (proc.exitCode !== null) {
      return { ok: false, detail: `process exited with code ${proc.exitCode} before ready` };
    }
    await delay(200);
  }
  return { ok: false, detail: `no app_ready within ${timeoutMs}ms` };
}

function scanForKnownBad(output) {
  // Filled in by Task 12
  return { findings: [], linesScanned: output.split('\n').length };
}

async function terminate(proc) {
  if (proc.exitCode !== null) return;
  if (platform() === 'win32') {
    spawn('taskkill', ['/PID', String(proc.pid), '/T'], { stdio: 'ignore' });
  } else {
    proc.kill('SIGTERM');
  }
  const exited = await Promise.race([
    new Promise((resolve) => proc.once('exit', () => resolve(true))),
    delay(SHUTDOWN_GRACE_MS).then(() => false),
  ]);
  if (!exited) {
    if (platform() === 'win32') {
      spawn('taskkill', ['/PID', String(proc.pid), '/T', '/F'], { stdio: 'ignore' });
    } else {
      proc.kill('SIGKILL');
    }
    await new Promise((resolve) => proc.once('exit', resolve));
  }
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

try {
  await main();
  writeFileSync(resultPath, JSON.stringify({ status: 'passed', checks }, null, 2));
} catch (err) {
  failed = true;
  failureReason = err instanceof Error ? err.message : String(err);
  if (proc && proc.exitCode === null) {
    try { await terminate(proc); } catch {}
  }
  writeFileSync(resultPath, JSON.stringify({ status: 'failed', reason: failureReason, checks }, null, 2));
}

if (failed) {
  console.error(`Process-smoke failed: ${failureReason}`);
  process.exit(1);
}
```

**Step 2: Add `test:e2e:smoke` script**

Edit `package.json`. In the `scripts` block, add:

```json
"test:e2e:smoke": "node e2e/process-smoke.mjs"
```

Maintain JSON formatting (comma placement). Re-verify the file parses with `node -e "JSON.parse(require('fs').readFileSync('package.json','utf8'))"`.

**Step 3: Commit**

```bash
git add e2e/process-smoke.mjs package.json
git commit -m "feat(e2e): add Layer 2 process-smoke harness skeleton

Cross-platform Node harness — launches binary, captures stdout/stderr,
waits for snk::smoke app_ready marker, holds for hold-window, terminates
gracefully. Scrape + screenshot logic added in subsequent commits.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 12: Fill in the known-bad-signature scraper

**Files:**
- Modify: `e2e/process-smoke.mjs` (replace `scanForKnownBad` stub)

**Step 1: Define the scrape patterns**

Replace the `scanForKnownBad` function body in `e2e/process-smoke.mjs`:

```javascript
function scanForKnownBad(output) {
  const lines = output.split('\n');
  const patterns = [
    {
      category: 'CSP violation',
      // The CLAUDE.md line-47 gotcha: WebView2 routes through http://
      // for asset.localhost / ipc.localhost in packaged builds; if CSP
      // lists only one scheme, the browser logs a Content Security
      // Policy violation.
      regex: /Content[- ]Security[- ]Policy/i,
    },
    {
      category: 'asset.localhost load failure',
      regex: /asset\.localhost.*(failed|refused|blocked|error)/i,
    },
    {
      category: 'ipc.localhost load failure',
      regex: /ipc\.localhost.*(failed|refused|blocked|error)/i,
    },
    {
      category: 'Rust panic',
      // panic format: `thread '<name>' panicked at ...`
      regex: /thread '[^']+' panicked/i,
    },
    {
      category: 'Tauri ACL rejection',
      // Tauri 2 ACL rejects unknown commands with this phrase
      regex: /not allowed by ACL|not allowed by the ACL/i,
    },
    {
      category: 'plugin setup failed',
      // main.rs emits this when the SafeSetupPlugin wrapper catches a
      // panic in a plugin's initialize()
      regex: /plugin setup panicked|plugin:setup-failed/i,
    },
  ];

  const findings = [];
  for (const line of lines) {
    for (const { category, regex } of patterns) {
      if (regex.test(line)) {
        findings.push({ category, evidence: line.trim().slice(0, 500) });
      }
    }
  }

  return { findings, linesScanned: lines.length };
}
```

**Step 2: Manually verify the patterns**

Construct a fake log and run the scraper against it as a sanity check:

```bash
node -e "
const m = require('./e2e/process-smoke.mjs');  // won't actually import; we want to test the regexes
" 2>&1 || true
```

Better — write a tiny ad-hoc test (do NOT commit this file):

```bash
node -e "
const lines = [
  'thread main panicked at src/main.rs:42',
  'Content Security Policy: The page settings blocked the loading of a resource',
  'Tauri command foo not allowed by ACL',
  'plugin setup panicked: snk-library failed to initialize',
  'asset.localhost connection refused',
];
const patterns = [
  /thread '?[^']*'? panicked/i,
  /Content[- ]Security[- ]Policy/i,
  /not allowed by ACL/i,
  /plugin setup panicked/i,
  /asset\.localhost.*(failed|refused|blocked|error)/i,
];
for (const line of lines) {
  for (const p of patterns) {
    if (p.test(line)) console.log('MATCH:', p.source, '<-', line);
  }
}
"
```

Expected: every fake line matches exactly one pattern. If a pattern is too loose (matches multiple unrelated lines) or too strict (misses its target), tighten/loosen and re-run.

**Step 3: Commit**

```bash
git add e2e/process-smoke.mjs
git commit -m "feat(e2e): scrape stdout/stderr for known-bad signatures

Patterns cover: CSP violations (line-47 gotcha), asset/ipc.localhost
load failures, Rust panics, Tauri ACL rejections, plugin setup panics
(via the SafeSetupPlugin wrapper in main.rs).

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 13: Add native screenshot + app-log copy to artifacts

**Files:**
- Modify: `e2e/process-smoke.mjs` (add screenshot helper + app-log copy after termination)

**Step 1: Add the screenshot helper**

Append to `e2e/process-smoke.mjs` (above the `try { await main(); ... }` block), and call from `main()` after `await delay(STEADY_HOLD_MS)`:

```javascript
function screenshotCommand() {
  const out = join(ARTIFACT_DIR, 'screenshot.png');
  if (platform() === 'darwin') {
    return { bin: 'screencapture', args: ['-x', out], outPath: out };
  }
  if (platform() === 'win32') {
    // PowerShell + System.Drawing; falls back to a black image if no
    // active display. Non-fatal either way.
    const ps = `
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bmp.Save('${out.replace(/\\/g, '\\\\')}', [System.Drawing.Imaging.ImageFormat]::Png)
`;
    return { bin: 'powershell', args: ['-NoProfile', '-Command', ps], outPath: out };
  }
  return null;
}

async function captureScreenshot() {
  const cmd = screenshotCommand();
  if (!cmd) {
    logCheck('screenshot', true, 'platform not supported; skipping');
    return;
  }
  await new Promise((resolve) => {
    const child = spawn(cmd.bin, cmd.args, { stdio: 'ignore' });
    child.on('exit', (code) => {
      if (code === 0 && existsSync(cmd.outPath)) {
        logCheck('screenshot', true, cmd.outPath);
      } else {
        logCheck('screenshot', true, `non-fatal capture failure (exit ${code})`);
      }
      resolve();
    });
    child.on('error', () => {
      logCheck('screenshot', true, `non-fatal: ${cmd.bin} not found`);
      resolve();
    });
  });
}
```

In `main()`, add right after `await delay(STEADY_HOLD_MS)`:

```javascript
await captureScreenshot();
```

**Step 2: Copy app-log dir into artifacts**

Add another helper, called from `main()` after `await terminate(proc)`:

```javascript
async function copyAppLogs() {
  // The app's tracing layer writes to `app_log_dir`, which on Windows is
  // typically %APPDATA%/com.snapper-keeper.app/logs and on macOS is
  // ~/Library/Logs/com.snapper-keeper.app. Since we override APPDATA /
  // LOCALAPPDATA / XDG_DATA_HOME to APP_DATA_DIR, look there first.
  const candidates = [
    join(APP_DATA_DIR, 'com.snapper-keeper.app', 'logs'),
    join(APP_DATA_DIR, 'logs'),
  ];
  const dest = join(ARTIFACT_DIR, 'app-logs');
  mkdirSync(dest, { recursive: true });
  let copied = 0;
  for (const src of candidates) {
    if (!existsSync(src)) continue;
    for (const entry of readdirSync(src)) {
      const srcPath = join(src, entry);
      try {
        if (statSync(srcPath).isFile()) {
          copyFileSync(srcPath, join(dest, entry));
          copied++;
        }
      } catch {}
    }
  }
  logCheck('app-logs copied', true, `${copied} file(s)`);
}
```

In `main()`, add right after `await terminate(proc)`:

```javascript
await copyAppLogs();
```

**Step 3: Verify locally (optional)**

If on an interactive desktop, run:

```bash
pnpm tauri build --debug
# Note the output binary path under target/debug/bundle/...
SNK_BINARY_PATH=<path-to-built-binary> node e2e/process-smoke.mjs
```

Expected: `result.json` exists with `status: passed`; `process-stdout.log`, `process-stderr.log`, `screenshot.png`, and `app-logs/` populate.

If this can't be run interactively, skip — CI will exercise it in Task 15.

**Step 4: Commit**

```bash
git add e2e/process-smoke.mjs
git commit -m "feat(e2e): native screenshot + app-log copy for process-smoke

macOS uses screencapture(1); Windows uses PowerShell System.Drawing.
Screenshot failure is non-fatal (hosted runners may lack an active
display session). App-log copy looks under the overridden APP_DATA_DIR.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 14: Cross-platform binary-path resolver script

The CI step needs to find the packaged binary in a platform-specific bundle dir (`target/release/bundle/nsis/...` on Windows; `target/release/bundle/macos/...` on macOS). Wrap that logic in a small script so the CI YAML stays clean.

**Files:**
- Create: `scripts/resolve-built-binary.sh` (bash, runs on both windows-latest via Git Bash and macos-latest)

**Step 1: Write the resolver**

Create `scripts/resolve-built-binary.sh`:

```bash
#!/usr/bin/env bash
# Resolves the path to the built snapper-keeper binary after
# `bash scripts/build-local.sh` or `pnpm tauri build` completes.
#
# Outputs the binary path to stdout. Exits non-zero with a diagnostic
# message on stderr if nothing is found.
#
# Used by the e2e-process-smoke CI job to set SNK_BINARY_PATH.

set -Eeuo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

OS_RAW="$(uname -s)"
case "$OS_RAW" in
  Darwin)
    # tauri build on macOS produces target/<triple>/release/bundle/macos/<name>.app
    # The executable inside the .app is <name>.app/Contents/MacOS/<name>
    candidates=(
      target/aarch64-apple-darwin/release/bundle/macos/*.app/Contents/MacOS/snapper-keeper-app
      target/x86_64-apple-darwin/release/bundle/macos/*.app/Contents/MacOS/snapper-keeper-app
      target/release/bundle/macos/*.app/Contents/MacOS/snapper-keeper-app
    )
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # NSIS bundle path; binary is target/<triple>/release/snapper-keeper-app.exe
    candidates=(
      target/x86_64-pc-windows-msvc/release/snapper-keeper-app.exe
      target/release/snapper-keeper-app.exe
    )
    ;;
  *)
    echo "resolve-built-binary: unsupported OS: $OS_RAW" >&2
    exit 1
    ;;
esac

for c in "${candidates[@]}"; do
  # Glob expansion may not match; use compgen to filter
  for match in $(compgen -G "$c" || true); do
    if [[ -x "$match" ]]; then
      echo "$match"
      exit 0
    fi
  done
done

echo "resolve-built-binary: no built binary found at any candidate path:" >&2
for c in "${candidates[@]}"; do
  echo "  $c" >&2
done
exit 1
```

Make it executable:

```bash
chmod +x scripts/resolve-built-binary.sh
```

**Step 2: Verify on a fresh build (if interactive desktop available)**

```bash
bash scripts/build-local.sh
bash scripts/resolve-built-binary.sh
```

Expected: outputs an absolute path to the binary, which exists and is executable. If the path doesn't match what `build-local.sh` produces, update the candidates list.

If you can't run interactively, verify by reading `scripts/build-local.sh` carefully and confirming the paths match its output (look for `BUNDLES=` and the bundle output directories).

**Step 3: Commit**

```bash
git add scripts/resolve-built-binary.sh
git commit -m "feat(e2e): resolve-built-binary.sh — locate packaged binary after build

Used by the e2e-process-smoke CI job to set SNK_BINARY_PATH after
build-local.sh produces the bundle. Per-OS candidate paths; exits
non-zero with a diagnostic if no candidate matches.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 15: Add `e2e-process-smoke` matrix job to CI

**Files:**
- Modify: `.github/workflows/ci.yml` (add new job)

**Step 1: Append the new job**

Open `.github/workflows/ci.yml`. Append a new job at the end of the `jobs:` map:

```yaml
  e2e-process-smoke:
    # Layer 2 of the e2e strategy — see
    # docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
    #
    # Builds the packaged binary (matching scripts/build-local.sh output),
    # launches it, waits for the snk::smoke app_ready marker, scrapes
    # stdout/stderr/app-log for known-bad signatures, captures a native
    # screenshot, terminates. No WebDriver — replaced the WebDriver-based
    # smoke previously in this workflow because runner-Edge / msedgedriver
    # version drift made it structurally flaky.
    needs: [lint-typecheck, rust-test]
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, macos-latest]
    timeout-minutes: 25
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
      - uses: pnpm/action-setup@d15e628ca66d93ee5f352c71671a7bc6a97af5c9 # v6.0.8
      - uses: actions/setup-node@48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e # v6.4.0
        with:
          node-version: 20
          cache: pnpm
      - uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8 # stable
      - uses: Swatinem/rust-cache@e18b497796c12c097a38f9edb9d0641fb99eee32 # v2

      - name: Install pnpm deps
        run: pnpm install --frozen-lockfile

      - name: Build frontend
        run: pnpm --filter @snk/app build

      - name: Build packaged binary (unsigned)
        shell: bash
        env:
          # Tell build-local.sh to skip signing — it gates on these env vars.
          # See scripts/build-local.sh for the gate logic.
          SNK_SKIP_SIGNING: '1'
        run: bash scripts/build-local.sh

      - name: Resolve built binary path
        id: resolve
        shell: bash
        run: |
          path="$(bash scripts/resolve-built-binary.sh)"
          echo "path=$path" >> "$GITHUB_OUTPUT"
          echo "Resolved binary: $path"

      - name: Run process-smoke
        shell: bash
        env:
          SNK_BINARY_PATH: ${{ steps.resolve.outputs.path }}
          E2E_ARTIFACT_DIR: ${{ github.workspace }}/e2e-artifacts
        run: |
          mkdir -p "$E2E_ARTIFACT_DIR"
          node e2e/process-smoke.mjs

      - name: Upload process-smoke artifacts
        if: always()
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          name: e2e-process-smoke-artifacts-${{ matrix.os }}
          path: ${{ github.workspace }}/e2e-artifacts
          if-no-files-found: error
```

**Step 2: Verify `scripts/build-local.sh` actually supports the `SNK_SKIP_SIGNING` env**

Read `scripts/build-local.sh`. If `SNK_SKIP_SIGNING` is NOT respected, two options:
1. **Add a `--no-sign` flag or env-gate** to `build-local.sh` (small edit; keeps the script as the single build entrypoint), commit as a separate precursor task.
2. **Use a different build command** in the CI step — call `pnpm tauri build` directly with a config overlay (`--config '{"bundle":{"...":"..."}}'`) that disables signing.

Pick option 1 if `build-local.sh` already has a recognizable signing gate (look for `codesign`, `notarize`, `signtool`); add the env check just above where signing runs. Pick option 2 if `build-local.sh` is tightly coupled to signing without an existing toggle.

Document the chosen path in the commit message.

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add e2e-process-smoke matrix job (windows-latest + macos-latest)

Builds the packaged binary via scripts/build-local.sh, runs
e2e/process-smoke.mjs against it. Uploads per-OS artifact bundles.
Replaces the removed e2e-windows-smoke WebDriver job.

timeout-minutes: 25 budget — tauri build takes ~10 min on hosted
runners; smoke + scrape adds ~30s; cache miss adds ~5 min.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

If Step 2 required a precursor commit (option 1 above), commit that FIRST with message:

```bash
git commit -m "feat(scripts): build-local.sh honors SNK_SKIP_SIGNING

Needed so CI's e2e-process-smoke job can produce an unsigned bundle
without code-signing certs in the environment.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 16: Update README

**Files:**
- Modify: `README.md` (replace removed WebDriver smoke section with new process-smoke description)

**Step 1: Add process-smoke description**

Open `README.md`. Find the CI summary section (where the WebDriver smoke text was removed in Task 2). Add a description of the new approach. Tone: factual, contributor-focused. Approximate content:

```markdown
### E2E process-smoke

The `e2e-process-smoke` CI job runs on every PR against `windows-latest` and `macos-latest`. It builds an unsigned packaged binary via `scripts/build-local.sh`, launches it, waits for the `snk::smoke` `app_ready` log marker (emitted from `main.rs`'s `RunEvent::Ready` handler), then scrapes stdout/stderr/app-log for known-bad signatures: CSP violations, asset-protocol load failures, Rust panics, Tauri ACL rejections, plugin setup failures.

Layer 1 of the same strategy runs in the `rust-test` job: each plugin crate has a `tests/command_acl_smoke.rs` that loads the plugin under `tauri::test::mock_builder` to catch ACL/permission misconfiguration without a real runtime.

UI interactivity (clicks, typing, search) is intentionally out of scope for the per-PR gate; see the design doc for the longer-term plan.
```

**Step 2: Verify**

```bash
# Eyeball the rendered markdown — easy to mess up section heading levels
grep -n '^#' README.md | head -30
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): describe e2e-process-smoke + Layer 1 ACL smoke

Replaces the removed WebDriver smoke section.

Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md
Issue: #47"
```

---

### Task 17: Local rust-test pass + push, monitor CI

**Step 1: Run the full Rust test suite locally**

```bash
cargo test --workspace --exclude snapper-keeper-app
```

Expected: PASS. The 7 new `command_acl_smoke` tests should each PASS. If any fails:
- Surface the failure to the user (don't suppress).
- If it's a real bug in main (e.g., the three-entries gotcha exists today for some plugin), file a separate issue and add `#[ignore]` to the failing test with a comment linking to the issue. Do NOT delete the test.

**Step 2: Push**

```bash
git push origin copilot/add-e2e-smoke-tests-per-os
```

**Step 3: Update PR title and body**

```bash
gh pr edit 144 --title "Replace WebDriver smoke with packaged-binary process-smoke (Layer 1 + Layer 2)"
```

Then update the body. Use a HEREDOC. The new body should describe:
- The pivot (link the issue comment from earlier)
- Layer 1 (per-plugin `command_acl_smoke.rs`)
- Layer 2 (`e2e-process-smoke` matrix job)
- What's removed (WebDriver smoke)
- Link to the design doc

```bash
gh pr edit 144 --body "$(cat <<'EOF'
Replaces PR #144's failing WebDriver smoke with a two-layer process-based approach. See [the pivot rationale on issue #47](https://github.com/ehartye/snapper-keeper/issues/47#issuecomment-4550538183) and [the full design](../blob/copilot/add-e2e-smoke-tests-per-os/docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md).

## Layer 1 — per-plugin ACL smoke (existing `rust-test` job)

Each command-shipping plugin crate (7 of them; `snk-pii` skipped — pure data) gets `tests/command_acl_smoke.rs` that loads the plugin under `tauri::test::mock_builder`. Catches the three-coordinated-entries gotcha (CLAUDE.md line 48) at unit-test speed.

## Layer 2 — packaged-binary process-smoke (new `e2e-process-smoke` matrix job)

Matrix `windows-latest` + `macos-latest`. Builds packaged binary via `scripts/build-local.sh`, launches it, waits for `snk::smoke` `app_ready` marker (emitted from `RunEvent::Ready`), scrapes stdout/stderr/app-log for CSP violations / panics / ACL rejections / plugin setup failures, captures a native screenshot, terminates. Uploads per-OS artifact bundles.

## Removed

- `e2e/windows-smoke.mjs` (WebDriver smoke script)
- `e2e-windows-smoke` job (5 consecutive structural failures in Edge-driver-pin step)
- `test:e2e:windows` package.json script
- README WebDriver section

## Out of scope

UI interactivity (clicks, typing, search). Deferred to a future workflow_dispatch / release-gated job.

Closes #47.
EOF
)"
```

**Step 4: Watch CI**

```bash
gh pr checks 144 --watch
```

Expected: all checks pass. Likely failure modes and triage:

- **Layer 1 fails on some plugin**: read the test output. If it's the gotcha biting now, surface to the user. If it's an API mismatch (e.g. `mock_builder` signature differs in this Tauri version), fix in-place — the plan's Task 1 spelled out the API; if it's wrong, the plan is wrong, fix it (this is the plan-as-source-of-truth pattern from CLAUDE.md).
- **Layer 2 build fails**: probably `build-local.sh` signing gate or path issue. Read the log; fix in `scripts/build-local.sh` or in the CI step.
- **Layer 2 smoke fails with steady-state timeout**: the binary may not be reaching `RunEvent::Ready` in CI. Inspect the uploaded `process-stdout.log` artifact — if the app is logging but never hitting `Ready`, the marker placement in Task 3 is wrong; move it to `setup(...)`'s Ok() return as a fallback.
- **Layer 2 smoke fails with a scrape match**: investigate the matched line. If it's a real bug, surface to user. If it's a false-positive on a benign log line, tighten the regex in Task 12.

**Step 5: If checks pass, commit the watch**

No commit needed; CI passing is the artifact.

If checks fail and you fix something, the fix follows the plan-as-source-of-truth pattern (CLAUDE.md): update this plan doc to reflect what was wrong, then commit the fix.

---

### Task 18: Address PR review feedback and merge

**Step 1: Request review or wait for it**

If the PR has reviewers assigned, ping them. Otherwise, wait for the user (Eric) to review.

**Step 2: Address feedback iteratively**

For each review comment:
1. Determine if it's a real issue or stylistic.
2. If real, fix in a new commit on the same branch. Commit message:
   ```
   fix(e2e): <what changed>

   Addresses review feedback on PR #144.
   ```
3. If stylistic / matter of opinion, push back politely on the thread with reasoning.
4. After each fix, push and re-watch CI.

**Step 3: Merge**

Once approved and CI green:

```bash
# Verify no surprises in the diff
gh pr diff 144 | head -200

# Merge (squash matches the project's convention — most prior PRs are squash-merged)
gh pr merge 144 --squash --delete-branch
```

**Step 4: Verify issue #47 closed**

```bash
gh issue view 47 --json state,number
```

Expected: `"state": "CLOSED"` (the PR body's `Closes #47` will auto-close it).

**Step 5: Clean up the local branch**

```bash
git checkout main
git pull --ff-only origin main
git branch -d copilot/add-e2e-smoke-tests-per-os || true
```

---

## Self-review notes

Checked against the spec:

- ✅ Layer 1 coverage: Tasks 1, 4–10 cover the 7 command-shipping plugins. `snk-pii` correctly excluded.
- ✅ Layer 2 harness: Tasks 11, 12, 13 build it incrementally (skeleton → scraper → screenshot/app-log).
- ✅ CI integration: Task 14 (binary resolver), Task 15 (matrix job), Task 17 (push + monitor).
- ✅ Removal of PR #144's WebDriver pieces: Task 2.
- ✅ `app_ready` smoke marker: Task 3 (explicit, decoupled from `logging::init` line).
- ✅ Spec risks addressed in plan:
  - Risk 1 (Tauri test API): Task 1 = pre-flight to validate
  - Risk 2 (`build-local.sh` on hosted runners): Task 15 Step 2 = explicit verify-and-adapt
  - Risk 3 (macOS screenshot): Task 13 = non-fatal screenshot failure
  - Risk 4 (steady-state marker): Task 3 = explicit marker
  - Risk 5 (job runtime budget): Task 15 = `timeout-minutes: 25`
  - Risk 6 (cache warm-up): mitigated by existing Swatinem/rust-cache
- ✅ Plan-as-source-of-truth: Task 17 Step 4 + Task 1 Step 4 explicitly call out fixing the plan in-place if reality diverges.

Spec sections without a task:
- Decisions log: implicit in the spec + PR; not a code change.
- Artifact strategy: covered by Task 13 (creating artifacts) + Task 15 (uploading them).

No gaps found.
