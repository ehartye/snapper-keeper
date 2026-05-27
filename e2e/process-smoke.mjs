// Layer 2 process-smoke harness.
//
// Launches the packaged snapper-keeper binary, waits for the
// `snk::smoke app_ready` log marker emitted from app/src-tauri/src/main.rs
// at the end of setup(), holds for a steady-state window, scrapes
// stdout/stderr for known-bad signatures, terminates gracefully,
// uploads artifacts.
//
// Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

import { spawn } from 'node:child_process';
import { createWriteStream, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { access } from 'node:fs/promises';
import { join } from 'node:path';
import { platform, tmpdir } from 'node:os';

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
  console.log(`${ok ? '[ok]' : '[fail]'} ${name}${detail ? `: ${detail}` : ''}`);
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForReady(collected, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const haystack = collected.stdout + '\n' + collected.stderr;
    if (haystack.includes('snk::smoke') && haystack.includes('app_ready')) {
      return { ok: true, detail: 'matched snk::smoke + app_ready in captured output' };
    }
    if (proc.exitCode !== null) {
      return { ok: false, detail: `process exited with code ${proc.exitCode} before ready` };
    }
    await delay(200);
  }
  return { ok: false, detail: `no app_ready within ${timeoutMs}ms` };
}

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
      regex: /thread '[^']+' panicked/i,
    },
    {
      category: 'Tauri ACL rejection',
      regex: /not allowed by ACL|not allowed by the ACL/i,
    },
    {
      category: 'plugin setup failed',
      // main.rs emits this via the SafeSetupPlugin wrapper when a
      // plugin's initialize() panics.
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

async function terminate() {
  if (!proc || proc.exitCode !== null) return;
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

  await delay(STEADY_HOLD_MS);
  logCheck('steady-state hold completed', true, `${STEADY_HOLD_MS}ms`);

  const scan = scanForKnownBad(collected.stdout + '\n' + collected.stderr);
  for (const finding of scan.findings) {
    logCheck(`scrape: ${finding.category}`, false, finding.evidence);
  }
  if (scan.findings.length > 0) {
    throw new Error(`scrape found ${scan.findings.length} bad-signature match(es)`);
  }
  logCheck('no bad-signature matches', true, `${scan.linesScanned} lines scanned`);

  await terminate();
  logCheck('graceful termination', true, '');
}

try {
  await main();
  writeFileSync(resultPath, JSON.stringify({ status: 'passed', checks }, null, 2));
} catch (err) {
  failed = true;
  failureReason = err instanceof Error ? err.message : String(err);
  try { await terminate(); } catch {}
  writeFileSync(resultPath, JSON.stringify({ status: 'failed', reason: failureReason, checks }, null, 2));
}

if (failed) {
  console.error(`Process-smoke failed: ${failureReason}`);
  process.exit(1);
}
