import { invoke } from '@tauri-apps/api/core';

import type { ClipboardItem, ListClipboardQuery, CaretPosition, SourceApp } from './types';
import type { ClipboardStatus } from './generated/clipboard-status';
import type { PermissionStatus } from './generated/permission-status';

export * from './types';
export * from './generated/errors';
export type { ClipboardStatus } from './generated/clipboard-status';
export type { PermissionStatus } from './generated/permission-status';

export const CLIPBOARD_HISTORY_EVENT = 'hotkey:clipboard-history';
export const CLIPBOARD_POPUP_SHOW_EVENT = 'clipboard-popup:show';
export const APP_BLOCKLIST_SETTING_KEY = 'clipboard.app_blocklist';

// Emitted when the clipboard watcher cannot open the OS clipboard; the popup
// renders an offline banner. CLIPBOARD_AVAILABLE_EVENT fires on recovery.
export const CLIPBOARD_UNAVAILABLE_EVENT = 'clipboard:unavailable';
export const CLIPBOARD_AVAILABLE_EVENT = 'clipboard:available';

export function listClipboardItems(query?: ListClipboardQuery): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>('plugin:snk-library|list_clipboard_items', { query });
}

export function getClipboardItem(id: string): Promise<ClipboardItem> {
  return invoke<ClipboardItem>('plugin:snk-library|get_clipboard_item', { id });
}

export function toggleClipboardPin(id: string, pinned: boolean): Promise<void> {
  return invoke<void>('plugin:snk-library|toggle_clipboard_pin', { id, pinned });
}

export function pasteItem(id: string, targetApp?: SourceApp): Promise<void> {
  return invoke<void>('plugin:snk-clipboard|paste_item', { id, targetApp });
}

export interface PopupContext {
  caret: CaretPosition;
  targetApp: SourceApp | null;
}

export function showPopup(): Promise<PopupContext> {
  return invoke<PopupContext>('plugin:snk-clipboard|show_popup');
}

export function detectFrontmostApp(): Promise<SourceApp | null> {
  return invoke<SourceApp | null>('plugin:snk-clipboard|detect_frontmost_app');
}

export function clipboardStatus(): Promise<ClipboardStatus> {
  return invoke<ClipboardStatus>('plugin:snk-clipboard|clipboard_status');
}

export function clipboardPermissionStatus(): Promise<PermissionStatus> {
  return invoke<PermissionStatus>('plugin:snk-clipboard|clipboard_permission_status');
}

export function openAccessibilitySettings(): Promise<void> {
  return invoke<void>('plugin:snk-clipboard|open_accessibility_settings');
}

// True when an invoke rejection is the typed "accessibility permission
// required" error from paste_item — lets the popup show an actionable banner
// instead of swallowing the failure.
export function isAccessibilityRequiredError(e: unknown): boolean {
  return (
    typeof e === 'object' &&
    e !== null &&
    (e as { kind?: string }).kind === 'accessibility-required'
  );
}
