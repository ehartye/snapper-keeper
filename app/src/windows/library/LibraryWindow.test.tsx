import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { LibraryWindow } from './LibraryWindow';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<LibraryWindow />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset().mockResolvedValue([]);
  });

  it('renders the header logotype and capture button', async () => {
    renderWithQuery(<LibraryWindow />);
    expect(screen.getByText('snapper')).toBeInTheDocument();
    expect(screen.getByText('keeper')).toBeInTheDocument();
    expect(screen.getByText(/Snap!/i)).toBeInTheDocument();
  });

  it('registers an onCloseRequested listener for hide-to-tray', async () => {
    renderWithQuery(<LibraryWindow />);
    await waitFor(() => {
      expect(getCurrentWindow().onCloseRequested).toHaveBeenCalled();
    });
  });

  it('Snap! button triggers a full-screen capture via the snk-capture plugin', async () => {
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|capture_full_screen') {
        return Promise.resolve({
          id: 'cap-1',
          file_path: 'x.png',
          annotated_path: null,
          width: 1,
          height: 1,
          source_app: null,
          source_window_title: null,
          monitor: null,
          created_at: 0,
          deleted_at: null,
          pinned: false,
        });
      }
      return Promise.resolve([]);
    });

    renderWithQuery(<LibraryWindow />);
    await act(async () => {
      fireEvent.click(screen.getByText(/Snap!/i));
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|capture_full_screen');
    });
  });

  it('subscribes to the capture and clipboard hotkey events', async () => {
    renderWithQuery(<LibraryWindow />);
    await waitFor(() => {
      const calls = vi.mocked(listen).mock.calls.map((c) => c[0]);
      for (const event of [
        'hotkey:capture-full-screen',
        'hotkey:capture-region',
        'hotkey:capture-window',
        'hotkey:capture-timed',
        'hotkey:clipboard-history',
      ]) {
        expect(calls).toContain(event);
      }
    });
  });

  it('region hotkey grabs a preview and shows the overlay', async () => {
    // Track which event the test wants to fire.
    let regionHandler: ((e: { payload: unknown }) => void) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'hotkey:capture-region') {
        regionHandler = handler as typeof regionHandler;
      }
      return Promise.resolve(() => {});
    });
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|grab_screen_preview') {
        return Promise.resolve({ path: '/tmp/p.png', width: 1, height: 1 });
      }
      return Promise.resolve([]);
    });

    renderWithQuery(<LibraryWindow />);
    await waitFor(() => expect(regionHandler).not.toBeNull());

    await act(async () => regionHandler!({ payload: undefined }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview');
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('capture-overlay');
    });
  });
});
