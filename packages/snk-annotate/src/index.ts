import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export * from './types';

// Editable annotation state, serialized into captures.annotation_state.
// The Rust side stores this verbatim — the schema lives here.
export interface AnnotationState {
  version: 1;
  shapes: unknown[]; // opaque to the binding; app's AnnotationShape[] passes through
  crop_region: { x: number; y: number; width: number; height: number } | null;
  crop_confirmed: boolean;
}

export function saveAnnotation(
  captureId: string,
  pngData: number[],
  state: AnnotationState,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|save_annotation', {
    captureId,
    pngData,
    stateJson: JSON.stringify(state),
  });
}
