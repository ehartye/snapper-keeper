import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

type Permission = string | { identifier: string; allow?: { path: string }[] };

const capabilityPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../../src-tauri/capabilities/settings.json',
);
const capability = JSON.parse(readFileSync(capabilityPath, 'utf8')) as {
  permissions: Permission[];
};
const openPathEntries = capability.permissions.filter((permission) =>
  typeof permission === 'string'
    ? permission === 'opener:allow-open-path'
    : permission.identifier === 'opener:allow-open-path',
);
const expectedPaths = [
  { path: '$APPDATA' },
  { path: '$APPDATA/**' },
  { path: '$APPLOG' },
  { path: '$APPLOG/**' },
];

describe('settings opener capability', () => {
  it('scopes open-path to the app data and log directories', () => {
    expect(openPathEntries).toHaveLength(1);
    expect(openPathEntries[0]).toEqual({
      identifier: 'opener:allow-open-path',
      allow: expectedPaths,
    });
  });

  it('allows no other open-path entries', () => {
    expect(openPathEntries).toEqual([
      { identifier: 'opener:allow-open-path', allow: expectedPaths },
    ]);
  });
});
