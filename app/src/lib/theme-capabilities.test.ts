import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

// Regression guard for the theme-per-window bug:
//
// App.tsx's WindowRouter calls useTheme() for EVERY window (before routing),
// and useTheme reads the current theme via the `get_theme` IPC command. So
// every window the app can render MUST have a capability granting that command
// — otherwise the read is ACL-rejected and the window silently falls back to
// DEFAULT_THEME. That is exactly what broke the annotate + clipboard-popup
// windows after the per-window capability split (#146): they kept rendering
// themed UI but lost the permission to read the persisted theme.
//
// This test ties the requirement to its cause: it derives the window list from
// App.tsx's router and the granted permissions from the capability files, so a
// future window (or a capability edit that drops the grant) is caught before
// release.

const here = dirname(fileURLToPath(import.meta.url)); // app/src/lib
const appRoot = resolve(here, '../..'); // app/
const capsDir = resolve(appRoot, 'src-tauri/capabilities');
const appTsx = resolve(appRoot, 'src/App.tsx');

const THEME_READ_PERMISSION = 'snk-library:allow-get-theme';

/** Window labels the app can render, parsed from App.tsx's WindowRouter switch. */
function routerWindowLabels(): string[] {
  const src = readFileSync(appTsx, 'utf8');
  const labels = [...src.matchAll(/case '([a-z0-9-]+)'/g)].map((m) => m[1]!);
  return [...new Set(labels)];
}

/** Map of window label → flattened permissions granted by its capability file(s). */
function permissionsByWindow(): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const file of readdirSync(capsDir).filter((f) => f.endsWith('.json'))) {
    const cap = JSON.parse(readFileSync(resolve(capsDir, file), 'utf8')) as {
      windows?: string[];
      permissions?: string[];
    };
    for (const w of cap.windows ?? []) {
      map.set(w, [...(map.get(w) ?? []), ...(cap.permissions ?? [])]);
    }
  }
  return map;
}

describe('theme capability contract', () => {
  it('every router window has a capability granting get_theme', () => {
    const perms = permissionsByWindow();
    const missing = routerWindowLabels().filter(
      (label) => !(perms.get(label) ?? []).includes(THEME_READ_PERMISSION),
    );
    expect(
      missing,
      `these windows render <App>/useTheme but cannot read the theme ` +
        `(missing ${THEME_READ_PERMISSION}): ${missing.join(', ')}`,
    ).toEqual([]);
  });
});
