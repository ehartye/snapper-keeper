import { useEffect, useCallback, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalPosition } from '@tauri-apps/api/dpi';
import { availableMonitors, getCurrentWindow } from '@tauri-apps/api/window';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
  grabScreenPreview,
} from '@snk/capture';
import { CLIPBOARD_HISTORY_EVENT, CLIPBOARD_POPUP_SHOW_EVENT, showPopup } from '@snk/clipboard';
import { getSetting } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';
import { CaptureGrid } from './CaptureGrid';
import { ClipboardList } from './ClipboardList';
import { FirstRunWizard } from './FirstRunWizard';
import { SearchBar } from './SearchBar';
import { Sidebar } from './Sidebar';
import type { SidebarSelection } from './Sidebar';

export function LibraryWindow() {
  const queryClient = useQueryClient();
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

  const showToolbar = useCallback(async (captureId: string) => {
    const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
    if (toolbar) {
      await toolbar.emit('toolbar:show', { captureId });
      await toolbar.show();
      await toolbar.setFocus();
    }
  }, []);

  const handleFullScreen = useCallback(async () => {
    try {
      const capture = await captureFullScreen();
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleRegion = useCallback(async () => {
    try {
      const preview = await grabScreenPreview();
      const overlay = await WebviewWindow.getByLabel('capture-overlay');
      if (overlay) {
        await overlay.emit('overlay:preview', { path: preview.path });
        await overlay.show();
        await overlay.setFocus();
      }
    } catch (e) {
      console.error('region overlay failed', e);
    }
  }, []);

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

      // Anchor the popup to the monitor that actually contains the caret —
      // window.screen only knows about the primary monitor and doesn't
      // exclude the taskbar / dock, so the old logic let the popup hang
      // into the taskbar area or off-screen on multi-monitor setups.
      const monitors = await availableMonitors();
      const fallback = monitors[0];
      if (!fallback) {
        // No monitors reported — last-resort placement at 0,0.
        await popup.setPosition(new LogicalPosition(0, 0));
        await popup.emit(CLIPBOARD_POPUP_SHOW_EVENT, {});
        await popup.show();
        await popup.setFocus();
        return;
      }
      const monitor =
        monitors.find(
          (m) =>
            pos.x >= m.position.x &&
            pos.x < m.position.x + m.size.width &&
            pos.y >= m.position.y &&
            pos.y < m.position.y + m.size.height,
        ) ?? fallback;

      // Work in logical pixels (the popup window is sized in logical px per
      // tauri.conf.json). Tauri's Monitor reports position/size in PHYSICAL
      // pixels, so divide by scaleFactor.
      const sf = monitor.scaleFactor;
      const caretX = pos.x / sf;
      const caretY = pos.y / sf;
      const monLeft = monitor.position.x / sf;
      const monTop = monitor.position.y / sf;
      const monRight = monLeft + monitor.size.width / sf;
      // Reserve ~50px at the bottom for the Windows taskbar / macOS dock —
      // Tauri's Monitor doesn't expose work-area separately, so this is a
      // heuristic. Conservative for the common case (taskbar 40-48px).
      const monBottom = monTop + monitor.size.height / sf - 50;

      const popupW = 380;
      const popupH = 480;
      const pad = 8;

      let x = caretX;
      let y = caretY + pad;

      // Flip above the caret if the popup would clip below.
      if (y + popupH > monBottom) y = caretY - popupH - pad;

      // Clamp horizontally to the monitor.
      if (x + popupW > monRight) x = monRight - popupW - pad;
      if (x < monLeft + pad) x = monLeft + pad;

      // Vertical safety: clamp to monitor; if the popup is somehow still too
      // tall to fit (sub-popup-height monitor), anchor to the bottom of the
      // work area so the filter input + first items stay visible.
      if (y < monTop + pad) y = monTop + pad;
      if (y + popupH > monBottom) y = monBottom - popupH;

      await popup.setPosition(new LogicalPosition(x, y));
      await popup.emit(CLIPBOARD_POPUP_SHOW_EVENT, {});
      await popup.show();
      await popup.setFocus();
    } catch (e) {
      console.error('clipboard popup failed', e);
    }
  }, []);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    const setup = async () => {
      unlisteners.push(await listen(CAPTURE_FULL_SCREEN_EVENT, handleFullScreen));
      unlisteners.push(await listen(CAPTURE_REGION_EVENT, handleRegion));
      unlisteners.push(await listen(CAPTURE_WINDOW_EVENT, handleWindow));
      unlisteners.push(await listen(CAPTURE_TIMED_EVENT, handleTimed));
      unlisteners.push(await listen(CLIPBOARD_HISTORY_EVENT, handleClipboardHistory));
    };
    setup().catch((e) => console.error('listen setup failed', e));
    return () => unlisteners.forEach((fn) => fn());
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
