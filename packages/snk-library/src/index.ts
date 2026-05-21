import { invoke } from '@tauri-apps/api/core';

import type { Capture, ListCapturesQuery, SearchResult, Tag } from './types';

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

export function listTags(): Promise<Tag[]> {
  return invoke<Tag[]>('plugin:snk-library|list_tags');
}

export function createTag(name: string, color: string): Promise<Tag> {
  return invoke<Tag>('plugin:snk-library|create_tag', { name, color });
}

export function updateTag(id: string, name: string, color: string): Promise<Tag> {
  return invoke<Tag>('plugin:snk-library|update_tag', { id, name, color });
}

export function deleteTag(id: string): Promise<void> {
  return invoke<void>('plugin:snk-library|delete_tag', { id });
}

export function assignTag(captureId: string, tagId: string): Promise<void> {
  return invoke<void>('plugin:snk-library|assign_tag', { captureId, tagId });
}

export function removeTag(captureId: string, tagId: string): Promise<void> {
  return invoke<void>('plugin:snk-library|remove_tag', { captureId, tagId });
}

export function listCaptureTags(captureId: string): Promise<Tag[]> {
  return invoke<Tag[]>('plugin:snk-library|list_capture_tags', { captureId });
}

export function getSetting(key: string): Promise<unknown | null> {
  return invoke('plugin:snk-library|get_setting', { key });
}

export function setSetting(key: string, value: unknown): Promise<void> {
  return invoke<void>('plugin:snk-library|set_setting', { key, value });
}
