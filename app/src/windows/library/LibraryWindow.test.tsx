import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { availableMonitors, cursorPosition, getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
import type { ScreenPreview } from '@snk/capture';
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
    const overlayHide = vi.fn().mockResolvedValue(undefined);
    const overlaySetPosition = vi.fn().mockResolvedValue(undefined);
    const overlaySetSize = vi.fn().mockResolvedValue(undefined);
    vi.mocked(WebviewWindow.getByLabel).mockImplementation(async (label: string) => {
      if (label === 'capture-overlay') {
        return {
          emit: overlayEmit,
          hide: overlayHide,
          isVisible: vi.fn().mockResolvedValue(true),
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
          width: 2880,
          height: 1800,
          token: 'tok-xyz',
          display: { id: 77, frame: { coordinateSpace: 'logical', x: -1440, y: 100, width: 1440, height: 900 } },
        });
      }
      return Promise.resolve([]);
    });

    renderLibraryWindow();
    await waitFor(() => expect(regionHandler).not.toBeNull());

    await act(async () => regionHandler!({ payload: undefined }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview');
      expect(overlayHide).toHaveBeenCalledOnce();
      const grabCall = mockedInvoke.mock.calls.findIndex(([name]) => name === 'plugin:snk-capture|grab_screen_preview');
      expect(overlayHide.mock.invocationCallOrder[0]).toBeLessThan(mockedInvoke.mock.invocationCallOrder[grabCall]!);
      expect(availableMonitors).not.toHaveBeenCalled();
      expect(cursorPosition).not.toHaveBeenCalled();
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('capture-overlay');
      expect(overlaySetPosition).toHaveBeenCalledWith(new LogicalPosition(-1440, 100));
      expect(overlaySetSize).toHaveBeenCalledWith(
        new LogicalSize(1440, 900),
      );
      expect(overlayEmit).toHaveBeenCalledWith('overlay:preview', {
        path: '/tmp/p.png',
        token: 'tok-xyz',
        width: 2880, height: 1800,
        display: { id: 77, frame: { coordinateSpace: 'logical', x: -1440, y: 100, width: 1440, height: 900 } },
      });
    });
  });

  it('coalesces region hotkeys until the complete preview display lifecycle finishes', async () => {
    let regionHandler: ((e: { payload: unknown }) => Promise<void>) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'hotkey:capture-region') regionHandler = handler as typeof regionHandler;
      return Promise.resolve(() => {});
    });
    const preview: ScreenPreview = {
      path: '/tmp/p.png', width: 2880, height: 1800, token: 'A',
      display: { id: 77, frame: { coordinateSpace: 'logical', x: 0, y: 0, width: 1440, height: 900 } },
    };
    let resolvePreview!: (value: ScreenPreview) => void;
    const pendingPreview = new Promise<ScreenPreview>(resolve => { resolvePreview = resolve; });
    let resolveFocus!: () => void;
    const pendingFocus = new Promise<void>(resolve => { resolveFocus = resolve; });
    const grab = vi.fn().mockImplementationOnce(() => pendingPreview).mockResolvedValue(preview);
    const focus = vi.fn().mockImplementationOnce(() => pendingFocus).mockResolvedValue(undefined);
    const show = vi.fn().mockResolvedValue(undefined);
    vi.mocked(WebviewWindow.getByLabel).mockResolvedValue({
      isVisible: vi.fn().mockResolvedValue(false), hide: vi.fn().mockResolvedValue(undefined),
      setPosition: vi.fn().mockResolvedValue(undefined), setSize: vi.fn().mockResolvedValue(undefined),
      emit: vi.fn().mockResolvedValue(undefined), show, setFocus: focus,
    } as unknown as WebviewWindow);
    mockedInvoke.mockImplementation(cmd => cmd === 'plugin:snk-capture|grab_screen_preview' ? grab() : Promise.resolve([]));
    renderLibraryWindow();
    await waitFor(() => expect(regionHandler).not.toBeNull());
    let firstRun!: Promise<void>;
    await act(async () => { firstRun = regionHandler!({ payload: undefined }); });
    await waitFor(() => expect(grab).toHaveBeenCalledOnce());
    await act(async () => { void regionHandler!({ payload: undefined }); });
    expect(grab).toHaveBeenCalledOnce();
    expect(show).not.toHaveBeenCalled();
    await act(async () => { resolvePreview(preview); });
    await waitFor(() => expect(focus).toHaveBeenCalledOnce());
    // Admission stays closed after grabbing, through geometry, show and focus.
    await act(async () => { void regionHandler!({ payload: undefined }); });
    expect(grab).toHaveBeenCalledOnce();
    await act(async () => { resolveFocus(); await firstRun; });
    await act(async () => { await regionHandler!({ payload: undefined }); });
    expect(grab).toHaveBeenCalledTimes(2);
    expect(show).toHaveBeenCalledTimes(2);
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
});
