import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Convert a library-relative file path to a webview-loadable URL.
 * The library root is the app data dir; Tauri's asset protocol serves it.
 */
export function captureAssetUrl(libraryRoot: string, relative: string): string {
  // Normalize separators
  const full = `${libraryRoot.replace(/[\\/]+$/, '')}/${relative.replace(/\\/g, '/')}`;
  return convertFileSrc(full);
}
