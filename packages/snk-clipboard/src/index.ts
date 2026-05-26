import { invoke } from '@tauri-apps/api/core';

import type { ClipboardItem, ListClipboardQuery, CaretPosition, SourceApp } from './types';

export * from './types';
export * from './generated/errors';

export const CLIPBOARD_HISTORY_EVENT = 'hotkey:clipboard-history';
export const CLIPBOARD_POPUP_SHOW_EVENT = 'clipboard-popup:show';
export const APP_BLOCKLIST_SETTING_KEY = 'clipboard.app_blocklist';

export function listClipboardItems(query?: ListClipboardQuery): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>('plugin:snk-library|list_clipboard_items', { query });
}

export function getClipboardItem(id: string): Promise<ClipboardItem> {
  return invoke<ClipboardItem>('plugin:snk-library|get_clipboard_item', { id });
}

export function toggleClipboardPin(id: string, pinned: boolean): Promise<void> {
  return invoke<void>('plugin:snk-library|toggle_clipboard_pin', { id, pinned });
}

export function pasteItem(id: string): Promise<void> {
  return invoke<void>('plugin:snk-clipboard|paste_item', { id });
}

export function showPopup(): Promise<CaretPosition> {
  return invoke<CaretPosition>('plugin:snk-clipboard|show_popup');
}

export function detectFrontmostApp(): Promise<SourceApp | null> {
  return invoke<SourceApp | null>('plugin:snk-clipboard|detect_frontmost_app');
}
