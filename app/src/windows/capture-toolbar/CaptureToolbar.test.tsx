import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { CaptureToolbar } from './CaptureToolbar';

describe('<CaptureToolbar />', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  });

  it('renders the four buttons', () => {
    render(<CaptureToolbar />);
    expect(screen.getByText('annotate')).toBeInTheDocument();
    expect(screen.getByText('copy')).toBeInTheDocument();
    expect(screen.getByText('save')).toBeInTheDocument();
    expect(screen.getByText('discard')).toBeInTheDocument();
  });

  it('subscribes to toolbar:show to learn the captureId', async () => {
    render(<CaptureToolbar />);
    await waitFor(() => {
      expect(listen).toHaveBeenCalledWith('toolbar:show', expect.any(Function));
    });
  });

  it('annotate opens the annotate window with the active captureId', async () => {
    // Simulate the listener firing by capturing the handler argument.
    let trigger: ((e: { payload: { captureId: string } }) => void) | null = null;
    vi.mocked(listen).mockImplementation((_event, handler) => {
      trigger = handler as typeof trigger;
      return Promise.resolve(() => {});
    });

    render(<CaptureToolbar />);
    await waitFor(() => expect(trigger).not.toBeNull());

    await act(async () => trigger!({ payload: { captureId: 'cap-1' } }));

    await act(async () => fireEvent.click(screen.getByText('annotate')));
    await waitFor(() => {
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('annotate');
    });
  });

  it('discard soft-deletes the active capture', async () => {
    let trigger: ((e: { payload: { captureId: string } }) => void) | null = null;
    vi.mocked(listen).mockImplementation((_event, handler) => {
      trigger = handler as typeof trigger;
      return Promise.resolve(() => {});
    });

    render(<CaptureToolbar />);
    await waitFor(() => expect(trigger).not.toBeNull());
    await act(async () => trigger!({ payload: { captureId: 'cap-1' } }));

    await act(async () => fireEvent.click(screen.getByText('discard')));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|soft_delete_capture', {
        id: 'cap-1',
      });
    });
  });

  it('Escape dismisses the toolbar via the current window hide()', async () => {
    render(<CaptureToolbar />);
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(getCurrentWindow().hide).toHaveBeenCalled();
    });
  });
});
