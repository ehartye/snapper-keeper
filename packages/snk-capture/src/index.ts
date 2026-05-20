import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export const CAPTURE_FULL_SCREEN_EVENT = 'hotkey:capture-full-screen';

export function captureFullScreen(): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_full_screen');
}
