import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  OCR_READY_EVENT,
  getOcrWords,
  ocrStatus,
  onOcrReady,
  type OcrStatus,
  type OcrWord,
} from './index';

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

describe('@snk/ocr event constants', () => {
  it('matches the Rust-side event name', () => {
    expect(OCR_READY_EVENT).toBe('ocr:ready');
  });
});

describe('@snk/ocr bindings', () => {
  beforeEach(() => {
    mockedInvoke.mockReset().mockResolvedValue(undefined);
    mockedListen.mockReset().mockResolvedValue(() => {});
  });

  it('getOcrWords camelCases the capture id and returns words', async () => {
    const sample: OcrWord[] = [
      { text: 'hello', bbox: { x: 0.1, y: 0.05, w: 0.08, h: 0.04 }, confidence: 0.97, line: 0 },
    ];
    mockedInvoke.mockResolvedValue(sample);
    const out = await getOcrWords('cap-abc');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-ocr|get_ocr_words', {
      captureId: 'cap-abc',
    });
    expect(out).toEqual(sample);
  });

  it('ocrStatus takes no args and returns backend metadata', async () => {
    const status: OcrStatus = {
      backend: 'Vision',
      version: 'Vision (macOS 15.2.0)',
      last_error: null,
    };
    mockedInvoke.mockResolvedValue(status);
    const s = await ocrStatus();
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-ocr|ocr_status');
    expect(s.backend).toBe('Vision');
    expect(s.last_error).toBeNull();
  });

  it('ocrStatus surfaces last_error with typed kind discriminator', async () => {
    const status: OcrStatus = {
      backend: 'none',
      version: 'unavailable',
      last_error: { kind: 'backend-unavailable', reason: 'no recognizer language' },
    };
    mockedInvoke.mockResolvedValue(status);
    const s = await ocrStatus();
    expect(s.last_error?.kind).toBe('backend-unavailable');
    expect(s.last_error?.reason).toBe('no recognizer language');
  });

  it('onOcrReady subscribes to ocr:ready and forwards payload', async () => {
    const handler = vi.fn();
    await onOcrReady(handler);
    expect(mockedListen).toHaveBeenCalledWith('ocr:ready', expect.any(Function));

    const tauriCallback = mockedListen.mock.calls[0]?.[1] as ((e: { payload: string }) => void) | undefined;
    expect(tauriCallback).toBeDefined();
    tauriCallback?.({ payload: 'cap-xyz' });
    expect(handler).toHaveBeenCalledWith('cap-xyz');
  });
});
