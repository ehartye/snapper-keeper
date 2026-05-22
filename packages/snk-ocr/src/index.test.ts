import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import { OCR_COMPLETED_EVENT, ocrStatus } from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/ocr bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('OCR_COMPLETED_EVENT matches the Rust-side event name', () => {
    expect(OCR_COMPLETED_EVENT).toBe('ocr:completed');
  });

  it('ocrStatus returns the status string', async () => {
    mockedInvoke.mockResolvedValue('running');
    const s = await ocrStatus();
    expect(s).toBe('running');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-ocr|ocr_status');
  });
});
