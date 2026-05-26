import { createWriteStream, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { access } from 'node:fs/promises';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

const WEBDRIVER_ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
const DRIVER_URL = process.env.TAURI_DRIVER_URL ?? 'http://127.0.0.1:4444';
const APP_BINARY_PATH = process.env.APP_BINARY_PATH;
const ARTIFACT_DIR = process.env.E2E_ARTIFACT_DIR ?? join(process.cwd(), 'e2e-artifacts');
const SCREENSHOT_PATH = join(ARTIFACT_DIR, 'library-window.png');
const TAURI_DRIVER_LOG_PATH = join(ARTIFACT_DIR, 'tauri-driver.log');
const RESULT_PATH = join(ARTIFACT_DIR, 'result.json');
const WEBDRIVER_REQUEST_TIMEOUT_MS = 30_000;

// Create artifact dir before try/catch so result.json can always be written on failure.
mkdirSync(ARTIFACT_DIR, { recursive: true });

let tauriDriver;
let tauriDriverLog;
let sessionId;
let failed = false;
let failureReason;

try {
  if (!APP_BINARY_PATH) {
    throw new Error('APP_BINARY_PATH is required');
  }
  await access(APP_BINARY_PATH);

  tauriDriverLog = createWriteStream(TAURI_DRIVER_LOG_PATH, { flags: 'a' });
  tauriDriver = spawn('tauri-driver', [], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: process.env,
  });
  tauriDriver.stdout.pipe(tauriDriverLog);
  tauriDriver.stderr.pipe(tauriDriverLog);

  sessionId = await createSession();
  const title = await webdriver('GET', `/session/${sessionId}/title`);
  if (!String(title.value ?? '').toLowerCase().includes('snapper-keeper')) {
    throw new Error(`Unexpected window title: ${title.value}`);
  }

  const input = await waitForElement(
    sessionId,
    'xpath',
    "//input[contains(@placeholder, 'search captures')]",
    15_000,
  );
  await webdriver('POST', `/session/${sessionId}/element/${input}/click`, {});
  const query = 'webdriver-smoke-query';
  await webdriver('POST', `/session/${sessionId}/element/${input}/value`, {
    text: query,
    value: [...query],
  });

  await waitForElement(sessionId, 'xpath', "//*[contains(text(),'No results')]", 15_000);
  await saveScreenshot(sessionId, SCREENSHOT_PATH);

  writeFileSync(
    RESULT_PATH,
    JSON.stringify(
      {
        status: 'passed',
        checks: [
          'window title includes snapper-keeper',
          'search box is available and accepts input',
          'search UI renders response state',
          'library screenshot captured',
        ],
      },
      null,
      2,
    ),
  );
} catch (error) {
  failed = true;
  failureReason = error instanceof Error ? error.message : String(error);
  if (sessionId && !existsSync(SCREENSHOT_PATH)) {
    try {
      await saveScreenshot(sessionId, SCREENSHOT_PATH);
    } catch {}
  }
  writeFileSync(RESULT_PATH, JSON.stringify({ status: 'failed', reason: failureReason }, null, 2));
} finally {
  if (sessionId) {
    try {
      await webdriver('DELETE', `/session/${sessionId}`);
    } catch {}
  }

  if (tauriDriver && tauriDriver.exitCode === null) {
    tauriDriver.kill();
    await new Promise((resolve) => tauriDriver.once('exit', resolve));
  }
  if (tauriDriverLog) {
    tauriDriverLog.end();
  }
}

if (failed) {
  throw new Error(`Windows E2E smoke failed: ${failureReason}`);
}

async function createSession() {
  const deadline = Date.now() + 30_000;
  let lastError = new Error('tauri-driver session startup timeout');
  while (Date.now() < deadline) {
    try {
      const response = await webdriver('POST', '/session', {
        capabilities: {
          alwaysMatch: {
            'tauri:options': {
              application: APP_BINARY_PATH,
            },
          },
        },
      });
      const id = response.sessionId ?? response.value?.sessionId;
      if (id) return id;
      throw new Error(`session id missing: ${JSON.stringify(response)}`);
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      await delay(500);
    }
  }
  throw lastError;
}

async function waitForElement(sessionId, using, value, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let lastError = new Error(`element not found: ${value}`);
  while (Date.now() < deadline) {
    try {
      const response = await webdriver('POST', `/session/${sessionId}/element`, { using, value });
      const elementId = response.value?.[WEBDRIVER_ELEMENT_KEY];
      if (elementId) return elementId;
      throw new Error(`missing element id: ${JSON.stringify(response)}`);
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      await delay(250);
    }
  }
  throw lastError;
}

async function saveScreenshot(sessionId, destinationPath) {
  const response = await webdriver('GET', `/session/${sessionId}/screenshot`);
  const base64 = response.value;
  if (!base64) {
    throw new Error('missing screenshot payload');
  }
  writeFileSync(destinationPath, Buffer.from(base64, 'base64'));
}

async function webdriver(method, path, body) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), WEBDRIVER_REQUEST_TIMEOUT_MS);

  let response;
  try {
    response = await fetch(`${DRIVER_URL}${path}`, {
      method,
      headers: body ? { 'content-type': 'application/json' } : undefined,
      body: body ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    });
  } catch (error) {
    if (error.name === 'AbortError') {
      throw new Error(
        `WebDriver ${method} ${path} timed out after ${WEBDRIVER_REQUEST_TIMEOUT_MS}ms`,
      );
    }
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }

  let json = null;
  try {
    json = await response.json();
  } catch {}

  if (!response.ok || json?.value?.error) {
    const detail = json?.value?.message ?? json?.value?.error ?? response.statusText;
    throw new Error(`WebDriver ${method} ${path} failed: ${detail}`);
  }
  return json;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
