import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { CaptureToolbar } from './CaptureToolbar';

function renderWithToolbarEvents() {
  let trigger: ((e: { payload: { captureId: string } }) => void) | undefined;
  vi.mocked(listen).mockImplementation((_event, handler) => {
    trigger = handler as typeof trigger;
    return Promise.resolve(() => {});
  });
  const result = render(<CaptureToolbar />);
  const show = (captureId: string) => {
    if (!trigger) throw new Error('toolbar listener was not registered');
    act(() => trigger!({ payload: { captureId } }));
  };
  return { ...result, show };
}

describe('<CaptureToolbar />', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    vi.mocked(listen).mockReset().mockResolvedValue(() => {});
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
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

  describe('copy', () => {
    it('copy calls copyCaptureImage with the active capture id', async () => {
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(invoke).toHaveBeenCalledWith('plugin:snk-clipboard|copy_capture_image', {
        captureId: 'cap-1',
      });
    });

    it('copy button is disabled while pending', async () => {
      vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(screen.getByText('copy')).toBeDisabled();
      fireEvent.click(screen.getByText('copy'));
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it('copy shows Copied after success', async () => {
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(screen.getByText('Copied ✓')).toBeInTheDocument();
    });

    it('copy dismisses 1500 ms after success', async () => {
      vi.useFakeTimers();
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      await act(async () => vi.advanceTimersByTimeAsync(1499));
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
      await act(async () => vi.advanceTimersByTimeAsync(1));
      expect(getCurrentWindow().hide).toHaveBeenCalledTimes(1);
    });

    it('copy shows Copy failed after rejection', async () => {
      const error = new Error('clipboard unavailable');
      vi.spyOn(console, 'error').mockImplementation(() => {});
      vi.mocked(invoke).mockRejectedValue(error);
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(screen.getByText('Copy failed')).toBeInTheDocument();
      expect(console.error).toHaveBeenCalledWith('copy failed', error);
    });

    it('copy stays open after a failure', async () => {
      vi.useFakeTimers();
      vi.spyOn(console, 'error').mockImplementation(() => {});
      vi.mocked(invoke).mockRejectedValue(new Error('clipboard unavailable'));
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
    });

    it('copy without a capture id shows Copy failed', async () => {
      render(<CaptureToolbar />);
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(screen.getByText('Copy failed')).toBeInTheDocument();
    });

    it('copy without a capture id calls no command', async () => {
      render(<CaptureToolbar />);
      await act(async () => fireEvent.click(screen.getByText('copy')));
      expect(invoke).not.toHaveBeenCalled();
    });

    it.each(['copied', 'failed'] as const)(
      'copy status resets on toolbar:show after %s',
      async (status) => {
        vi.spyOn(console, 'error').mockImplementation(() => {});
        if (status === 'failed') vi.mocked(invoke).mockRejectedValue(new Error('failed'));
        const { show } = renderWithToolbarEvents();
        show('cap-1');
        await act(async () => fireEvent.click(screen.getByText('copy')));
        expect(screen.getByText(status === 'copied' ? 'Copied ✓' : 'Copy failed')).toBeInTheDocument();
        show('cap-2');
        expect(screen.getByText('copy')).toBeEnabled();
      },
    );

    it('copy result is discarded after toolbar:show', async () => {
      vi.useFakeTimers();
      let resolveCopy!: () => void;
      vi.mocked(invoke).mockReturnValue(new Promise<void>((resolve) => { resolveCopy = resolve; }));
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      show('cap-2');
      await act(async () => resolveCopy());
      expect(screen.getByText('copy')).toBeEnabled();
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
    });

    it('discards a failed copy result after toolbar:show', async () => {
      let rejectCopy!: (error: Error) => void;
      vi.spyOn(console, 'error').mockImplementation(() => {});
      vi.mocked(invoke).mockReturnValue(new Promise<void>((_resolve, reject) => {
        rejectCopy = reject;
      }));
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      show('cap-2');
      await act(async () => rejectCopy(new Error('stale failure')));
      expect(screen.getByText('copy')).toBeEnabled();
      expect(console.error).not.toHaveBeenCalled();
    });

    it('dismiss timer is cancelled on toolbar:show', async () => {
      vi.useFakeTimers();
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      show('cap-2');
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
      expect(screen.getByText('copy')).toBeInTheDocument();
    });

    it('cancels the dismissal timer on unmount', async () => {
      vi.useFakeTimers();
      const { show, unmount } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      unmount();
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
    });

    it('cancels an earlier dismissal when a retry fails', async () => {
      vi.useFakeTimers();
      vi.spyOn(console, 'error').mockImplementation(() => {});
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      vi.mocked(invoke).mockRejectedValue(new Error('clipboard unavailable'));
      await act(async () => fireEvent.click(screen.getByText('Copied ✓')));
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(screen.getByText('Copy failed')).toBeInTheDocument();
      expect(getCurrentWindow().hide).not.toHaveBeenCalled();
    });

    it('does not apply a pending copy result after Escape', async () => {
      vi.useFakeTimers();
      let resolveCopy!: () => void;
      vi.mocked(invoke).mockReturnValue(new Promise<void>((resolve) => { resolveCopy = resolve; }));
      const { show } = renderWithToolbarEvents();
      show('cap-1');
      await act(async () => fireEvent.click(screen.getByText('copy')));
      await act(async () => fireEvent.keyDown(window, { key: 'Escape' }));
      await act(async () => resolveCopy());
      await act(async () => vi.advanceTimersByTimeAsync(2000));
      expect(screen.getByText('copy')).toBeInTheDocument();
      expect(getCurrentWindow().hide).toHaveBeenCalledTimes(1);
    });
  });
});
