import { invoke } from '@tauri-apps/api/core';

import type { Capture, ListCapturesQuery, SearchResult } from './types';

export * from './types';

export function listCaptures(query?: ListCapturesQuery): Promise<Capture[]> {
  return invoke<Capture[]>('plugin:snk-library|list_captures', { query });
}

export function getCapture(id: string): Promise<Capture> {
  return invoke<Capture>('plugin:snk-library|get_capture', { id });
}

export function softDeleteCapture(id: string): Promise<void> {
  return invoke<void>('plugin:snk-library|soft_delete_capture', { id });
}

export function searchLibrary(query: string, limit?: number): Promise<SearchResult[]> {
  return invoke<SearchResult[]>('plugin:snk-library|search_library', { query, limit });
}
