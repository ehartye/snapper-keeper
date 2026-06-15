import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { availableMonitors, cursorPosition, getCurrentWindow } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { ModalProvider } from '../../components/Modal';
import { LibraryWindow } from './LibraryWindow';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<LibraryWindow />', () => {
  const renderLibraryWindow = () =>
    renderWithQuery(
      <ModalProvider>
        <LibraryWindow />
      </ModalProvider>,
    );

  beforeEach(() => {
    mockedInvoke.mockReset().mockResolvedValue([]);
    const existing = document.getElementById('modal-root');
    if (!existing) {
      const root = document.createElement('div');
      root.id = 'modal-root';
      document.body.appendChild(root);
    }
  });

  it('renders the header logotype and capture button', async () => {
    renderLibraryWindow();
    expect(screen.getByText('snapper')).toBeInTheDocument();
    expect(screen.getByText('keeper')).toBeInTheDocument();
    expect(screen.getByText(/Snap!/i)).toBeInTheDocument();
  });

  it('registers an onCloseRequested listener for hide-to-tray', async () => {
    renderLibraryWindow();
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

    renderLibraryWindow();
    await act(async () => {
      fireEvent.click(screen.getByText(/Snap!/i));
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|capture_full_screen');
    });
  });

  it('subscribes to the capture and clipboard hotkey events', async () => {
    renderLibraryWindow();
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

  it('region hotkey grabs a preview and emits overlay:preview with path+token', async () => {
    let regionHandler: ((e: { payload: unknown }) => void) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'hotkey:capture-region') {
        regionHandler = handler as typeof regionHandler;
      }
      return Promise.resolve(() => {});
    });

    const overlayEmit = vi.fn().mockResolvedValue(undefined);
    const overlaySetPosition = vi.fn().mockResolvedValue(undefined);
    const overlaySetSize = vi.fn().mockResolvedValue(undefined);
    vi.mocked(WebviewWindow.getByLabel).mockImplementation(async (label: string) => {
      if (label === 'capture-overlay') {
        return {
          emit: overlayEmit,
          setPosition: overlaySetPosition,
          setSize: overlaySetSize,
          show: vi.fn().mockResolvedValue(undefined),
          setFocus: vi.fn().mockResolvedValue(undefined),
        } as unknown as WebviewWindow;
      }
      return null;
    });
    vi.mocked(availableMonitors).mockResolvedValue([
      {
        name: 'Primary',
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 1,
      },
      {
        name: 'Secondary',
        position: { x: 1920, y: 0 },
        size: { width: 2560, height: 1440 },
        scaleFactor: 1.5,
      },
    ]);
    vi.mocked(cursorPosition).mockResolvedValue({ x: 2100, y: 100 });

    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|grab_screen_preview') {
        return Promise.resolve({
          path: '/tmp/p.png',
          width: 1,
          height: 1,
          token: 'tok-xyz',
        });
      }
      return Promise.resolve([]);
    });

    renderLibraryWindow();
    await waitFor(() => expect(regionHandler).not.toBeNull());

    await act(async () => regionHandler!({ payload: undefined }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview', {
        monitorId: 1,
      });
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('capture-overlay');
      expect(overlaySetPosition).toHaveBeenCalledWith(expect.objectContaining({ x: 1920, y: 0 }));
      expect(overlaySetSize).toHaveBeenCalledWith(
        expect.objectContaining({ width: 2560, height: 1440 }),
      );
      expect(overlayEmit).toHaveBeenCalledWith('overlay:preview', {
        path: '/tmp/p.png',
        token: 'tok-xyz',
        monitorId: 1,
        scaleFactor: 1.5,
      });
    });
  });

  it('shows a plugin startup failure modal with copy diagnostics action', async () => {
    let pluginSetupFailedHandler: ((e: { payload: unknown }) => void) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'plugin:setup-failed') {
        pluginSetupFailedHandler = handler as typeof pluginSetupFailedHandler;
      }
      return Promise.resolve(() => {});
    });

    renderLibraryWindow();
    await waitFor(() => expect(pluginSetupFailedHandler).not.toBeNull());

    await act(async () => {
      pluginSetupFailedHandler!({
        payload: {
          pluginName: 'snk-library',
          panicMessage: 'simulated setup panic',
          diagnosticsMarkdown:
            '## Plugin setup panic\n- Plugin: `snk-library`\n- Panic: `simulated setup panic`',
        },
      });
    });

    expect(screen.getByText('Plugin "snk-library" failed to start')).toBeInTheDocument();
    expect(
      screen.getByText('Plugin snk-library failed to start — please file a bug.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy diagnostics' })).toBeInTheDocument();
  });

  it('shows raw-dev guidance when screen-recording-permission-denied on a raw-dev runtime', async () => {
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|capture_full_screen') {
        return Promise.reject({ kind: 'screen-recording-permission-denied' });
      }
      if (cmd === 'capture_runtime_status') {
        return Promise.resolve('raw-dev');
      }
      return Promise.resolve([]);
    });

    renderLibraryWindow();
    await act(async () => {
      fireEvent.click(screen.getByText(/Snap!/i));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('capture_runtime_status');
      expect(
        screen.getByText(/stop this session and use pnpm dev:mac-capture/i),
      ).toBeInTheDocument();
    });
  });

  it('shows raw-dev guidance when backend-unavailable is raised on a raw-dev runtime', async () => {
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|capture_full_screen') {
        return Promise.reject({
          kind: 'backend-unavailable',
          data: { detail: 'ScreenCaptureKit returned no shareable displays' },
        });
      }
      if (cmd === 'capture_runtime_status') {
        return Promise.resolve('raw-dev');
      }
      return Promise.resolve([]);
    });

    renderLibraryWindow();
    await act(async () => {
      fireEvent.click(screen.getByText(/Snap!/i));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('capture_runtime_status');
      expect(
        screen.getByText(/stop this session and use pnpm dev:mac-capture/i),
      ).toBeInTheDocument();
    });
  });
});
