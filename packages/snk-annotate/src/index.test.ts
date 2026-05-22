import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import { saveAnnotation } from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/annotate bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('saveAnnotation forwards captureId, PNG bytes, and serialized state', async () => {
    mockedInvoke.mockResolvedValue({ id: 'cap-1' });
    const png = [137, 80, 78, 71];
    const state = {
      version: 1 as const,
      shapes: [],
      crop_region: null,
      crop_confirmed: false,
    };
    const result = await saveAnnotation('cap-1', png, state);
    expect(result).toEqual({ id: 'cap-1' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-annotate|save_annotation', {
      captureId: 'cap-1',
      pngData: png,
      stateJson: JSON.stringify(state),
    });
  });
});
