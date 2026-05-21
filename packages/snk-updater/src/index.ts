import { invoke } from '@tauri-apps/api/core';

import type { UpdateStatus } from './types';

export * from './types';

export function checkForUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|check_for_update');
}

export function getUpdateStatus(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>('plugin:snk-updater|get_update_status');
}
