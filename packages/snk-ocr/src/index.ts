import { invoke } from '@tauri-apps/api/core';

export * from './types';

export const OCR_COMPLETED_EVENT = 'ocr:completed';

export function ocrStatus(): Promise<string> {
  return invoke<string>('plugin:snk-ocr|ocr_status');
}
