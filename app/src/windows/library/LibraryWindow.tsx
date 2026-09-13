import { useEffect, useCallback, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { clipboardPosition, overlayGeometry } from '../../lib/displayGeometry';
import { availableMonitors, getCurrentWindow } from '@tauri-apps/api/window';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
  grabScreenPreview,
  openScreenRecordingSettings,
} from '@snk/capture';
import { CLIPBOARD_HISTORY_EVENT, CLIPBOARD_POPUP_SHOW_EVENT, showPopup } from '@snk/clipboard';
import { getSetting } from '@snk/library';

import { useModal } from '../../components/Modal';
import { queryKeys } from '../../lib/queryKeys';
import { CaptureGrid } from './CaptureGrid';
import { ClipboardList } from './ClipboardList';
import { FirstRunWizard } from './FirstRunWizard';
import { SearchBar } from './SearchBar';
import { Sidebar } from './Sidebar';
import type { SidebarSelection } from './Sidebar';

interface PluginSetupFailedPayload {
  pluginName: string;
  panicMessage: string;
  diagnosticsMarkdown: string;
}

export function LibraryWindow() {
  const regionOpening = useRef(false);
  const queryClient = useQueryClient();
  const modal = useModal();
  const [selection, setSelection] = useState<SidebarSelection>({
    type: 'captures',
    label: 'All',
    query: {},
  });

  const firstRun = useQuery({
    queryKey: queryKeys.settings.one('firstrun.completed'),
    queryFn: () => getSetting('firstrun.completed'),
  });

  const [wizardDismissed, setWizardDismissed] = useState(false);
  const showWizard = !wizardDismissed && firstRun.data !== true;

  // X button → hide to tray instead of destroying the webview. The library
  // window owns the global hotkey listeners (capture, clipboard popup, etc.);
  // if it gets destroyed those events have nowhere to go and Ctrl+Shift+V
  // stops working. Quit via tray menu only.
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('library close listener failed', e));
    return () => cleanup?.();
  }, []);

  const refreshCaptures = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: ['captures'] });
  }, [queryClient]);

  // Refresh the gallery whenever ANY capture lands — fresh screenshots,
  // crop derivations (snk-annotate derive_capture), or anything else that
  // emits capture:saved. The hotkey handlers below also call refreshCaptures
  // directly, but this listener catches everything they don't (e.g. saves
  // from the annotate window).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen('capture:saved', () => {
      void refreshCaptures();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('library capture:saved listener failed', e));
    return () => unlisten?.();
  }, [refreshCaptures]);

  useEffect(() => {
    let cancelled = false;
    let unlisteners: (() => void)[] = [];
    const setup = async () => {
      const fn = await listen<PluginSetupFailedPayload>('plugin:setup-failed', ({ payload }) => {
        modal.custom({
          title: `Plugin "${payload.pluginName}" failed to start`,
          render: ({ close }) => (
            <div className="space-y-4">
              <p className="text-sm text-fg">
                Plugin {payload.pluginName} failed to start — please file a bug.
              </p>
              <pre className="text-xs whitespace-pre-wrap break-words p-2 bg-surface border border-border">
                {payload.panicMessage}
              </pre>
              <div className="flex justify-end gap-2">
                <button
                  className="font-display text-[11px] uppercase tracking-widest px-3 py-1.5 border-2 border-border bg-surface hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[1px_1px_0_0_var(--border)] transition-transform"
                  onClick={() => {
                    void navigator.clipboard.writeText(payload.diagnosticsMarkdown).catch((e) => {
                      console.error('copy diagnostics failed', e);
                    });
                  }}
                >
                  Copy diagnostics
                </button>
                <button
                  className="font-display text-[11px] uppercase tracking-widest px-3 py-1.5 border-2 border-border bg-primary text-bg hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[1px_1px_0_0_var(--border)] transition-transform"
                  onClick={close}
                >
                  OK
                </button>
              </div>
            </div>
          ),
        });
      });
      if (cancelled) {
        fn();
      } else {
        unlisteners = [fn];
      }
    };
    setup().catch((e) => console.error('library plugin:setup-failed listener failed', e));
    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [modal]);

  const showToolbar = useCallback(async (captureId: string) => {
    const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
    if (toolbar) {
      await toolbar.emit('toolbar:show', { captureId });
      await toolbar.show();
      await toolbar.setFocus();
    }
  }, []);

  const showScreenRecordingAlert = useCallback(() => {
    void (async () => {
      let isRawDev = false;
      try {
        const status = await invoke<string>('capture_runtime_status');
        isRawDev = status === 'raw-dev';
      } catch {
        // If runtime classification fails, fall through to the standard alert.
      }

      if (isRawDev) {
        modal.alert({
          title: 'Screen Recording Unavailable in Dev Mode',
          body: 'You are running via tauri dev. Stop this session and use pnpm dev:mac-capture instead — it builds and signs a proper .app bundle so macOS grants Screen Recording permission.',
        });
      } else {
        modal.confirm({
          title: 'Screen Recording Permission Required',
          body: 'Snapper Keeper needs Screen Recording permission to capture your screen. Open System Settings → Privacy & Security → Screen Recording and enable it for this app, then try again.',
          confirmLabel: 'Open Settings',
          cancelLabel: 'Dismiss',
          onConfirm: () => {
            openScreenRecordingSettings().catch((e) =>
              console.error('open screen recording settings failed', e),
            );
          },
        });
      }
    })();
  }, [modal]);

  const handleFullScreen = useCallback(async () => {
    try {
      const capture = await captureFullScreen();
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e: unknown) {
      if (typeof e === 'object' && e !== null && 'kind' in e && e.kind === 'screen-recording-permission-denied') {
        showScreenRecordingAlert();
        return;
      }
      console.error('capture failed', e);
    }
  }, [refreshCaptures, showToolbar, showScreenRecordingAlert]);

  const handleRegion = useCallback(async () => {
    // Coalesce hotkeys for the entire opening lifecycle. Serializing only the
    // native grabs still lets an earlier caller show its overlay during the next
    // grab. A later hotkey can replace the preview once display/focus finishes.
    if (regionOpening.current) return;
    regionOpening.current = true;
    try {
      const overlay = await WebviewWindow.getByLabel('capture-overlay');
      if (overlay) {
        // A repeated hotkey must not snapshot the previous overlay, even when
        // hide_own_windows is disabled. Only replacements need this extra settle.
        if (await overlay.isVisible()) {
          await overlay.hide();
          await new Promise(resolve => setTimeout(resolve, 150));
        }
        const preview = await grabScreenPreview();
        const geometry = overlayGeometry(preview.display.frame);
        await overlay.setPosition(geometry.position);
        await overlay.setSize(geometry.size);
        await overlay.emit('overlay:preview', preview);
        await overlay.show();
        await overlay.setFocus();
      }
    } catch (e: unknown) {
      if (typeof e === 'object' && e !== null && 'kind' in e && e.kind === 'screen-recording-permission-denied') {
        showScreenRecordingAlert();
        return;
      }
      console.error('region overlay failed', e);
    } finally {
      regionOpening.current = false;
    }
  }, [showScreenRecordingAlert]);

  const handleWindow = useCallback(async () => {
    try {
      const { listCapturableWindows, captureWindow } = await import('@snk/capture');
      const windows = await listCapturableWindows();
      const target = windows.find(
        (w) => !w.app_name.includes('snapper-keeper') && w.title.length > 0,
      );
      if (!target) {
        console.warn('no capturable window found');
        return;
      }
      const capture = await captureWindow(target.id);
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('window capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleTimed = useCallback(async () => {
    setTimeout(async () => {
      try {
        const capture = await captureFullScreen();
        await refreshCaptures();
        await showToolbar(capture.id);
      } catch (e) {
        console.error('timed capture failed', e);
      }
    }, 5000);
  }, [refreshCaptures, showToolbar]);

  const handleClipboardHistory = useCallback(async () => {
    try {
      const pos = await showPopup();
      const popup = await WebviewWindow.getByLabel('clipboard-popup');
      if (!popup) return;

      const monitors = await availableMonitors();
      await popup.setPosition(clipboardPosition(pos, monitors));
      await popup.emit(CLIPBOARD_POPUP_SHOW_EVENT, {});
      await popup.show();
      await popup.setFocus();
    } catch (e) {
      console.error('clipboard popup failed', e);
    }
  }, []);

  useEffect(() => {
    // `listen()` is async; the previous synchronous-cleanup version
    // races React StrictMode's setup→cleanup→setup pattern in dev,
    // ending up with TWO active listeners per event after StrictMode
    // settles. The `cancelled` flag lets a late-arriving setup
    // self-clean when it discovers the effect has already been torn
    // down. Without this, every Ctrl+Shift+4 fires handleRegion
    // twice in dev.
    let cancelled = false;
    let unlisteners: (() => void)[] = [];
    const setup = async () => {
      const fns = [
        await listen(CAPTURE_FULL_SCREEN_EVENT, handleFullScreen),
        await listen(CAPTURE_REGION_EVENT, handleRegion),
        await listen(CAPTURE_WINDOW_EVENT, handleWindow),
        await listen(CAPTURE_TIMED_EVENT, handleTimed),
        await listen(CLIPBOARD_HISTORY_EVENT, handleClipboardHistory),
      ];
      if (cancelled) {
        fns.forEach((fn) => fn());
      } else {
        unlisteners = fns;
      }
    };
    setup().catch((e) => console.error('listen setup failed', e));
    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [handleFullScreen, handleRegion, handleWindow, handleTimed, handleClipboardHistory]);

  return (
    <div className="h-full bg-bg text-fg">
      {showWizard && !firstRun.isLoading && (
        <FirstRunWizard
          onComplete={() => {
            setWizardDismissed(true);
            queryClient.invalidateQueries({
              queryKey: queryKeys.settings.one('firstrun.completed'),
            });
          }}
        />
      )}
      <main className="h-full flex">
        <Sidebar selection={selection} onSelect={setSelection} />
        <div className="flex-1 flex flex-col min-w-0">
          <header className="px-5 py-3 border-b border-border flex items-center gap-4">
            <h1 className="font-display text-xl leading-none">
              <span className="holo-shimmer bg-clip-text text-transparent">snapper</span>
              <span className="text-fg-muted">/</span>
              <span>keeper</span>
            </h1>
            <div className="flex-1 max-w-md">
              <SearchBar onSelectClipboard={() => setSelection({ type: 'clipboard' })} />
            </div>
            <button
              className="font-display text-[11px] uppercase tracking-widest px-4 py-2 bg-primary text-bg border-2 border-border shadow-[3px_3px_0_0_var(--border)] hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[1px_1px_0_0_var(--border)] active:translate-x-1 active:translate-y-1 active:shadow-none transition-transform"
              onClick={handleFullScreen}
            >
              Snap!
            </button>
          </header>
          <section className="flex-1 overflow-auto p-5">
            {selection.type === 'captures' ? (
              <CaptureGrid query={selection.query} />
            ) : (
              <ClipboardList />
            )}
          </section>
        </div>
      </main>
    </div>
  );
}
