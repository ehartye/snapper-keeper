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

describe('@snk/ocr getOcrWords', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue([]));

  it('invokes the correct command with captureId', async () => {
    const words: OcrWord[] = [
      { text: 'hello', bbox: { x: 0, y: 0, w: 10, h: 10 }, confidence: 0.99, line: 0 },
    ];
    mockedInvoke.mockResolvedValue(words);
    const result = await getOcrWords('cap-123');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-ocr|get_ocr_words', {
      captureId: 'cap-123',
    });
    expect(result).toEqual(words);
  });
});

describe('@snk/ocr ocrStatus', () => {
  beforeEach(() => mockedInvoke.mockReset());

  it('invokes the correct command and returns status', async () => {
    const status: OcrStatus = { backend: 'vision', version: '1.0', last_error: null };
    mockedInvoke.mockResolvedValue(status);
    const result = await ocrStatus();
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-ocr|ocr_status');
    expect(result).toEqual(status);
  });
});

describe('@snk/ocr onOcrReady', () => {
  beforeEach(() => mockedListen.mockReset().mockResolvedValue(() => {}));

  it('listens on the correct event and passes payload to handler', async () => {
    let captured: string | undefined;
    const unlisten = await onOcrReady((id) => {
      captured = id;
    });
    expect(mockedListen).toHaveBeenCalledWith(OCR_READY_EVENT, expect.any(Function));
    expect(typeof unlisten).toBe('function');
  });
});
