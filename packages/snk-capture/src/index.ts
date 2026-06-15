import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';
import type { WindowInfo } from './types';

export const CAPTURE_FULL_SCREEN_EVENT = 'hotkey:capture-full-screen';
export const CAPTURE_REGION_EVENT = 'hotkey:capture-region';
export const CAPTURE_WINDOW_EVENT = 'hotkey:capture-window';
export const CAPTURE_TIMED_EVENT = 'hotkey:capture-timed';
export * from './generated/errors';

export function captureFullScreen(): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_full_screen');
}

export function captureWindow(windowId: number): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_window', { windowId });
}

export function captureRegion(
  monitorId: number,
  x: number,
  y: number,
  w: number,
  h: number,
  scaleFactor: number,
  previewToken?: string,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_region', {
    request: {
      monitorId,
      x,
      y,
      w,
      h,
      scaleFactor,
      ...(previewToken ? { previewToken } : {}),
    },
  });
}

export function listCapturableWindows(): Promise<WindowInfo[]> {
  return invoke<WindowInfo[]>('plugin:snk-capture|list_capturable_windows');
}

export interface ScreenPreview {
  path: string;
  width: number;
  height: number;
  token: string;
  displayFrame: {
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactor: number;
  } | null;
  displayIndex: number | null;
}

export function grabScreenPreview(monitorId?: number): Promise<ScreenPreview> {
  if (monitorId === undefined) {
    return invoke<ScreenPreview>('plugin:snk-capture|grab_screen_preview');
  }
  return invoke<ScreenPreview>('plugin:snk-capture|grab_screen_preview', { monitorId });
}

export interface CursorPosition {
  x: number;
  y: number;
}

export function captureCursorPosition(): Promise<CursorPosition> {
  return invoke<CursorPosition>('plugin:snk-capture|capture_cursor_position');
}

export function capturePermissionStatus(): Promise<boolean> {
  return invoke<boolean>('plugin:snk-capture|capture_permission_status');
}

export function openScreenRecordingSettings(): Promise<void> {
  return invoke<void>('plugin:snk-capture|open_screen_recording_settings');
}

export type { WindowInfo } from './types';
