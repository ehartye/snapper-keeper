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

// Crop-save path: creates a brand-new capture row + PNG, inheriting
// source metadata from the parent and emitting capture:saved on the
// Rust side so OCR runs automatically. The frontend is responsible for
// computing the new image's dimensions and for shifting shape
// coordinates so they're relative to the new image's origin (see
// app/src/windows/annotate/cropDerivation.ts).
export function deriveCapture(
  parentId: string,
  pngData: number[],
  width: number,
  height: number,
  state: AnnotationState,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|derive_capture', {
    parentId,
    pngData,
    width,
    height,
    stateJson: JSON.stringify(state),
  });
}
