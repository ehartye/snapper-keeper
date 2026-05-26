import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
  captureWindow,
  captureRegion,
  listCapturableWindows,
  grabScreenPreview,
} from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/capture event constants', () => {
  it('match the Rust-side event names', () => {
    expect(CAPTURE_FULL_SCREEN_EVENT).toBe('hotkey:capture-full-screen');
    expect(CAPTURE_REGION_EVENT).toBe('hotkey:capture-region');
    expect(CAPTURE_WINDOW_EVENT).toBe('hotkey:capture-window');
    expect(CAPTURE_TIMED_EVENT).toBe('hotkey:capture-timed');
  });
});

describe('@snk/capture bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('captureFullScreen takes no args', async () => {
    await captureFullScreen();
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|capture_full_screen');
  });

  it('captureWindow camelCases the id', async () => {
    await captureWindow(42);
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|capture_window', {
      windowId: 42,
    });
  });

  it('captureRegion sends monitor + rect', async () => {
    await captureRegion(0, 10, 20, 300, 400);
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|capture_region', {
      monitorId: 0,
      x: 10,
      y: 20,
      w: 300,
      h: 400,
    });
  });

  it('listCapturableWindows takes no args', async () => {
    mockedInvoke.mockResolvedValue([]);
    await listCapturableWindows();
    expect(mockedInvoke).toHaveBeenCalledWith(
      'plugin:snk-capture|list_capturable_windows',
    );
  });

  it('grabScreenPreview returns the preview struct including token', async () => {
    mockedInvoke.mockResolvedValue({
      path: '/tmp/x.png',
      width: 800,
      height: 600,
      token: 'tok-abc',
    });
    const p = await grabScreenPreview();
    expect(p.path).toBe('/tmp/x.png');
    expect(p.width).toBe(800);
    expect(p.height).toBe(600);
    expect(p.token).toBe('tok-abc');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview');
  });

  it('grabScreenPreview forwards an optional monitor id', async () => {
    await grabScreenPreview(2);
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview', {
      monitorId: 2,
    });
  });
});
