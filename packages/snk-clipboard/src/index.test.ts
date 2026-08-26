import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import {
  CLIPBOARD_HISTORY_EVENT,
  CLIPBOARD_POPUP_SHOW_EVENT,
  listClipboardItems,
  getClipboardItem,
  toggleClipboardPin,
  pasteItem,
  showPopup,
  detectFrontmostApp,
} from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/clipboard event constants', () => {
  it('match the Rust-side event names', () => {
    expect(CLIPBOARD_HISTORY_EVENT).toBe('hotkey:clipboard-history');
    expect(CLIPBOARD_POPUP_SHOW_EVENT).toBe('clipboard-popup:show');
  });
});

describe('@snk/clipboard bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('listClipboardItems forwards query', async () => {
    mockedInvoke.mockResolvedValue([]);
    await listClipboardItems({ limit: 25, filter: 'foo' });
    expect(mockedInvoke).toHaveBeenCalledWith(
      'plugin:snk-library|list_clipboard_items',
      { query: { limit: 25, filter: 'foo' } },
    );
  });

  it('listClipboardItems allows omitted query', async () => {
    mockedInvoke.mockResolvedValue([]);
    await listClipboardItems();
    expect(mockedInvoke).toHaveBeenCalledWith(
      'plugin:snk-library|list_clipboard_items',
      { query: undefined },
    );
  });

  it('getClipboardItem sends id', async () => {
    mockedInvoke.mockResolvedValue({ id: 'x' });
    await getClipboardItem('x');
    expect(mockedInvoke).toHaveBeenCalledWith(
      'plugin:snk-library|get_clipboard_item',
      { id: 'x' },
    );
  });

  it('toggleClipboardPin sends id + pinned', async () => {
    await toggleClipboardPin('x', true);
    expect(mockedInvoke).toHaveBeenCalledWith(
      'plugin:snk-library|toggle_clipboard_pin',
      { id: 'x', pinned: true },
    );
  });

  it('pasteItem routes through the clipboard plugin', async () => {
    await pasteItem('x');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|paste_item', {
      id: 'x',
      targetApp: undefined,
    });
  });

  it('pasteItem forwards an optional target app', async () => {
    await pasteItem('x', {
      identifier: 'com.apple.TextEdit',
      display_name: 'TextEdit',
      kind: 'macos_bundle_id',
    });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|paste_item', {
      id: 'x',
      targetApp: {
        identifier: 'com.apple.TextEdit',
        display_name: 'TextEdit',
        kind: 'macos_bundle_id',
      },
    });
  });

  it('showPopup returns caret + target app context', async () => {
    mockedInvoke.mockResolvedValue({
      caret: { x: 100, y: 200 },
      targetApp: {
        identifier: 'com.apple.TextEdit',
        display_name: 'TextEdit',
        kind: 'macos_bundle_id',
      },
    });
    const ctx = await showPopup();
    expect(ctx).toEqual({
      caret: { x: 100, y: 200 },
      targetApp: {
        identifier: 'com.apple.TextEdit',
        display_name: 'TextEdit',
        kind: 'macos_bundle_id',
      },
    });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|show_popup');
  });

  it('detectFrontmostApp routes through the clipboard plugin', async () => {
    await detectFrontmostApp();
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|detect_frontmost_app');
  });
});
