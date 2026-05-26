import { describe, expect, it } from 'vitest';
import type { BlocklistEntry } from '@snk/clipboard';

import { readEntries } from './ClipboardSettings';

describe('readEntries', () => {
  it('returns empty array for non-array inputs', () => {
    expect(readEntries(null)).toEqual([]);
    expect(readEntries(undefined)).toEqual([]);
    expect(readEntries('not-an-array')).toEqual([]);
    expect(readEntries(42)).toEqual([]);
    expect(readEntries({})).toEqual([]);
    expect(readEntries({ not: 'an array' })).toEqual([]);
  });

  it('returns valid BlocklistEntry[] unchanged', () => {
    const input: BlocklistEntry[] = [
      { identifier: '1Password.exe', display_name: '1Password', kind: 'windows_exe' },
      {
        identifier: 'com.bitwarden.desktop',
        display_name: 'Bitwarden',
        kind: 'macos_bundle_id',
      },
    ];
    expect(readEntries(input)).toEqual(input);
  });

  it('filters out entries with missing identifier', () => {
    const input = [
      { display_name: 'No id', kind: 'windows_exe' },
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ]);
  });

  it('filters out entries with missing display_name', () => {
    const input = [
      { identifier: 'no-display.exe', kind: 'windows_exe' },
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ]);
  });

  it('filters out entries with non-string identifier', () => {
    const input = [
      { identifier: 123, display_name: 'Numeric id', kind: 'windows_exe' },
      { identifier: null, display_name: 'Null id', kind: 'windows_exe' },
      { identifier: ['array.exe'], display_name: 'Array id', kind: 'windows_exe' },
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ]);
  });

  it('filters out entries with non-string display_name', () => {
    const input = [
      { identifier: 'a.exe', display_name: 42, kind: 'windows_exe' },
      { identifier: 'b.exe', display_name: null, kind: 'windows_exe' },
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ]);
  });

  it('filters out entries with invalid kind', () => {
    const input = [
      { identifier: 'a.exe', display_name: 'A', kind: 'linux_appimage' },
      { identifier: 'b.exe', display_name: 'B', kind: 'unknown' },
      { identifier: 'c.exe', display_name: 'C' }, // missing kind entirely
      { identifier: 'd.exe', display_name: 'D', kind: null },
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
      { identifier: 'kept2', display_name: 'Kept2', kind: 'macos_bundle_id' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
      { identifier: 'kept2', display_name: 'Kept2', kind: 'macos_bundle_id' },
    ]);
  });

  it('filters out null and non-object entries', () => {
    const input = [
      null,
      undefined,
      'a string entry',
      42,
      true,
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ];
    expect(readEntries(input)).toEqual([
      { identifier: 'kept.exe', display_name: 'Kept', kind: 'windows_exe' },
    ]);
  });

  it('preserves entry order', () => {
    const input: BlocklistEntry[] = [
      { identifier: 'c.exe', display_name: 'C', kind: 'windows_exe' },
      { identifier: 'a.exe', display_name: 'A', kind: 'windows_exe' },
      { identifier: 'b.exe', display_name: 'B', kind: 'windows_exe' },
    ];
    expect(readEntries(input).map((e) => e.identifier)).toEqual([
      'c.exe',
      'a.exe',
      'b.exe',
    ]);
  });

  it('returns empty array for empty array input', () => {
    expect(readEntries([])).toEqual([]);
  });
});
