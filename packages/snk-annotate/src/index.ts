import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export * from './types';

export function saveAnnotation(captureId: string, pngData: number[]): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|save_annotation', {
    captureId,
    pngData,
  });
}
