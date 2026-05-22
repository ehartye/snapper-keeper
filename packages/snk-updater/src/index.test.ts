import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import { checkForUpdate, getUpdateStatus } from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/updater bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('checkForUpdate returns the status struct', async () => {
    mockedInvoke.mockResolvedValue({ kind: 'idle' });
    const s = await checkForUpdate();
    expect(s).toEqual({ kind: 'idle' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-updater|check_for_update');
  });

  it('getUpdateStatus returns the status struct', async () => {
    mockedInvoke.mockResolvedValue({ kind: 'available', version: '1.2.3' });
    const s = await getUpdateStatus();
    expect(s).toEqual({ kind: 'available', version: '1.2.3' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-updater|get_update_status');
  });
});
