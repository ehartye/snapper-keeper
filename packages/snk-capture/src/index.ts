import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';
import type { WindowInfo } from './types';

export const CAPTURE_FULL_SCREEN_EVENT = 'hotkey:capture-full-screen';
export const CAPTURE_REGION_EVENT = 'hotkey:capture-region';
export const CAPTURE_WINDOW_EVENT = 'hotkey:capture-window';
export const CAPTURE_TIMED_EVENT = 'hotkey:capture-timed';

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
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_region', { monitorId, x, y, w, h });
}

export function listCapturableWindows(): Promise<WindowInfo[]> {
  return invoke<WindowInfo[]>('plugin:snk-capture|list_capturable_windows');
}

export interface ScreenPreview {
  path: string;
  width: number;
  height: number;
  token: string;
}

export function grabScreenPreview(): Promise<ScreenPreview> {
  return invoke<ScreenPreview>('plugin:snk-capture|grab_screen_preview');
}

export type { WindowInfo } from './types';
