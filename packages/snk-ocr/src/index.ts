import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export const OCR_READY_EVENT = 'ocr:ready';

export interface BBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface OcrWord {
  text: string;
  bbox: BBox;
  confidence: number;
  line: number;
}

export interface OcrStatus {
  backend: string;
  version: string;
  last_error: { kind: string; [key: string]: unknown } | null;
}

export function getOcrWords(captureId: string): Promise<OcrWord[]> {
  return invoke<OcrWord[]>('plugin:snk-ocr|get_ocr_words', { captureId });
}

export function ocrStatus(): Promise<OcrStatus> {
  return invoke<OcrStatus>('plugin:snk-ocr|ocr_status');
}

export function onOcrReady(handler: (captureId: string) => void): Promise<UnlistenFn> {
  return listen<string>(OCR_READY_EVENT, (e) => handler(e.payload));
}
