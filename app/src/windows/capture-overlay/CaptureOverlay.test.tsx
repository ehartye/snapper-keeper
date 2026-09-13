import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { ScreenPreview } from '@snk/capture';
import { CaptureOverlay } from './CaptureOverlay';

const preview = (token: string): ScreenPreview => ({
  path: '/preview.png',
  token,
  width: 300,
  height: 150,
  display: {
    id: 77,
    frame: { coordinateSpace: 'logical', x: -1440, y: 100, width: 200, height: 100 },
  },
});
let receive: (event: { payload: ScreenPreview }) => void;

beforeEach(() => {
  vi.mocked(listen).mockImplementation((_name, handler) => {
    receive = handler as typeof receive;
    return Promise.resolve(() => {});
  });
  vi.mocked(invoke).mockResolvedValue({ id: 'capture' });
});

function drag(container: HTMLElement) {
  fireEvent.mouseDown(container.firstElementChild!, { clientX: 20, clientY: 20 });
  fireEvent.mouseMove(container.firstElementChild!, { clientX: 60, clientY: 40 });
  fireEvent.mouseUp(container.firstElementChild!, { clientX: 60, clientY: 40 });
}

describe('preview snapshot readiness', () => {
  it('rejects dragging before load, then crops actual rendered PNG pixels with its token', async () => {
    const { container } = render(<CaptureOverlay />);
    await act(async () => receive({ payload: preview('A') }));
    await act(async () => drag(container));
    expect(invoke).not.toHaveBeenCalled();
    const img = container.querySelector('img')!;
    vi.spyOn(img, 'getBoundingClientRect').mockReturnValue({
      left: 10,
      top: 10,
      width: 200,
      height: 100,
    } as DOMRect);
    fireEvent.load(img);
    await act(async () => drag(container));
    expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|capture_region', {
      previewToken: 'A',
      x: 15,
      y: 15,
      w: 60,
      h: 30,
    });
  });

  it('new tokens clear selection and an old image load cannot unlock the new preview', async () => {
    const { container } = render(<CaptureOverlay />);
    await act(async () => receive({ payload: preview('A') }));
    const oldImage = container.querySelector('img')!;
    fireEvent.load(oldImage);
    fireEvent.mouseDown(container.firstElementChild!, { clientX: 20, clientY: 20 });
    await act(async () => receive({ payload: preview('B') }));
    expect(container.querySelector('.border-blue-400')).toBeNull();
    fireEvent.load(oldImage);
    await act(async () => drag(container));
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByText('Loading preview... | Esc to cancel')).toBeInTheDocument();
  });

  it('does not show an old save toolbar over a newly arrived preview', async () => {
    let finishSave!: (value: { id: string }) => void;
    vi.mocked(invoke).mockImplementation(
      () =>
        new Promise((resolve) => {
          finishSave = resolve;
        }),
    );
    const { container } = render(<CaptureOverlay />);
    await act(async () => receive({ payload: preview('A') }));
    const img = container.querySelector('img')!;
    vi.spyOn(img, 'getBoundingClientRect').mockReturnValue({
      left: 10,
      top: 10,
      width: 200,
      height: 100,
    } as DOMRect);
    fireEvent.load(img);
    await act(async () => drag(container));
    expect(invoke).toHaveBeenCalledOnce();
    await act(async () => receive({ payload: preview('B') }));
    await act(async () => finishSave({ id: 'saved-A' }));
    expect(WebviewWindow.getByLabel).not.toHaveBeenCalled();
    expect(container.querySelector('img')!.src).toContain('v=B');
  });

  it('explains an expired snapshot instead of silently losing the selection', async () => {
    vi.mocked(invoke).mockRejectedValue({ kind: 'stale-preview' });
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { container } = render(<CaptureOverlay />);
    await act(async () => receive({ payload: preview('A') }));
    const img = container.querySelector('img')!;
    vi.spyOn(img, 'getBoundingClientRect').mockReturnValue({
      left: 10,
      top: 10,
      width: 200,
      height: 100,
    } as DOMRect);
    fireEvent.load(img);
    await act(async () => drag(container));
    expect(screen.getByText(/This preview expired/)).toBeInTheDocument();
    consoleError.mockRestore();
  });
});
