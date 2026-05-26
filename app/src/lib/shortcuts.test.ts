import { describe, expect, it } from 'vitest';

import { formatShortcutForPlatform } from './shortcuts';

describe('formatShortcutForPlatform', () => {
  it('uses Cmd on macOS', () => {
    expect(formatShortcutForPlatform('CmdOrCtrl+Shift+4', true)).toBe('Cmd+Shift+4');
    expect(formatShortcutForPlatform('Ctrl+Shift+V', true)).toBe('Cmd+Shift+V');
  });

  it('uses Ctrl on non-mac platforms', () => {
    expect(formatShortcutForPlatform('CmdOrCtrl+Shift+4', false)).toBe('Ctrl+Shift+4');
    expect(formatShortcutForPlatform('Ctrl+Shift+V', false)).toBe('Ctrl+Shift+V');
  });
});
