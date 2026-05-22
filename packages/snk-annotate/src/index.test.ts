import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import { saveAnnotation, deriveCapture } from './index';

const mockedInvoke = vi.mocked(invoke);

const cleanState = {
  version: 1 as const,
  shapes: [],
  crop_region: null,
  crop_confirmed: false,
};

describe('@snk/annotate bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('saveAnnotation forwards captureId, PNG bytes, and serialized state', async () => {
    mockedInvoke.mockResolvedValue({ id: 'cap-1' });
    const png = [137, 80, 78, 71];
    const result = await saveAnnotation('cap-1', png, cleanState);
    expect(result).toEqual({ id: 'cap-1' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-annotate|save_annotation', {
      captureId: 'cap-1',
      pngData: png,
      stateJson: JSON.stringify(cleanState),
    });
  });

  it('deriveCapture forwards parentId, PNG bytes, dimensions, and serialized state', async () => {
    mockedInvoke.mockResolvedValue({ id: 'cap-derived' });
    const png = [137, 80, 78, 71];
    const result = await deriveCapture('cap-parent', png, 200, 150, cleanState);
    expect(result).toEqual({ id: 'cap-derived' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-annotate|derive_capture', {
      parentId: 'cap-parent',
      pngData: png,
      width: 200,
      height: 150,
      stateJson: JSON.stringify(cleanState),
    });
  });
});
