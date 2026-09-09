import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { availableMonitors, cursorPosition } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { showCaptureToolbar, toolbarPositionForCursor } from './captureToolbar';

const navigatorWithUAData = navigator as Navigator & {
  userAgentData?: { platform?: string };
};

function mockNavigatorPlatform(platform: string, userAgent: string) {
  Object.defineProperty(window.navigator, 'platform', {
    configurable: true,
    value: platform,
  });
  Object.defineProperty(window.navigator, 'userAgent', {
    configurable: true,
    value: userAgent,
  });
  Object.defineProperty(navigatorWithUAData, 'userAgentData', {
    configurable: true,
    value: { platform },
  });
}

describe('capture toolbar placement', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    vi.mocked(cursorPosition).mockReset().mockResolvedValue({ x: 1000, y: 400 });
    vi.mocked(availableMonitors).mockReset().mockResolvedValue([
      {
        name: 'Primary',
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 2,
      },
    ]);
    mockNavigatorPlatform('Win32', 'Windows');
  });

  it('positions the toolbar near the cursor before showing it', async () => {
    const toolbar = {
      setPosition: vi.fn().mockResolvedValue(undefined),
      emit: vi.fn().mockResolvedValue(undefined),
      show: vi.fn().mockResolvedValue(undefined),
      setFocus: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(WebviewWindow.getByLabel).mockResolvedValue(toolbar as never);

    await showCaptureToolbar('cap-1');

    expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('capture-toolbar');
    expect(toolbar.setPosition).toHaveBeenCalledWith(expect.objectContaining({ x: 580, y: 424 }));
    expect(toolbar.emit).toHaveBeenCalledWith('toolbar:show', { captureId: 'cap-1' });
    expect(toolbar.show).toHaveBeenCalled();
    expect(toolbar.setFocus).toHaveBeenCalled();
  });

  it('clamps the toolbar to the active monitor bounds', () => {
    const position = toolbarPositionForCursor(
      [
        {
          name: 'Secondary',
          position: { x: 1920, y: 0 },
          size: { width: 2560, height: 1440 },
          scaleFactor: 2,
          workArea: {
            position: { x: 1920, y: 0 },
            size: { width: 2560, height: 1440 },
          },
        },
      ],
      { x: 4400, y: 100 },
    );

    expect(position).toEqual({ x: 3616, y: 124 });
  });

  it('keeps the toolbar on a secondary monitor when the cursor is there', () => {
    const position = toolbarPositionForCursor(
      [
        {
          name: 'Primary',
          position: { x: 0, y: 0 },
          size: { width: 1920, height: 1080 },
          scaleFactor: 2,
        },
        {
          name: 'Secondary',
          position: { x: 1920, y: 0 },
          size: { width: 2560, height: 1440 },
          scaleFactor: 2,
        },
      ],
      { x: 3000, y: 300 },
    );

    expect(position.x).toBeGreaterThanOrEqual(1920);
  });

  it('uses the native macOS capture cursor position instead of the JS cursor API', async () => {
    mockNavigatorPlatform('MacIntel', 'Mac OS X');
    vi.mocked(cursorPosition).mockResolvedValue({ x: 100, y: 100 });
    vi.mocked(availableMonitors).mockResolvedValue([
      {
        name: 'Primary',
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 2,
      },
      {
        name: 'Secondary',
        position: { x: 1920, y: 0 },
        size: { width: 2560, height: 1440 },
        scaleFactor: 2,
      },
    ]);
    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === 'plugin:snk-capture|capture_cursor_position') {
        return Promise.resolve({ x: 3000, y: 300 });
      }
      return Promise.resolve(undefined);
    });
    const toolbar = {
      setPosition: vi.fn().mockResolvedValue(undefined),
      emit: vi.fn().mockResolvedValue(undefined),
      show: vi.fn().mockResolvedValue(undefined),
      setFocus: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(WebviewWindow.getByLabel).mockResolvedValue(toolbar as never);

    await showCaptureToolbar('cap-2');

    expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|capture_cursor_position');
    expect(toolbar.setPosition).toHaveBeenCalledWith(expect.objectContaining({ x: 2580, y: 324 }));
  });
});
