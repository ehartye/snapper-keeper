import { useEffect, useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalPosition } from '@tauri-apps/api/dpi';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
} from '@snk/capture';
import { CLIPBOARD_HISTORY_EVENT, CLIPBOARD_POPUP_SHOW_EVENT, showPopup } from '@snk/clipboard';

import { CaptureGrid } from './CaptureGrid';
import { ClipboardList } from './ClipboardList';
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
    const overlay = await WebviewWindow.getByLabel('capture-overlay');
    if (overlay) {
      await overlay.show();
      await overlay.setFocus();
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
      if (popup) {
        await popup.setPosition(new LogicalPosition(pos.x, pos.y));
        await popup.emit(CLIPBOARD_POPUP_SHOW_EVENT, {});
        await popup.show();
        await popup.setFocus();
      }
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
    <main className="h-full flex">
      <Sidebar selection={selection} onSelect={setSelection} />
      <div className="flex-1 flex flex-col min-w-0">
        <header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
          <h1 className="text-sm font-semibold">snapper-keeper</h1>
          <div className="flex-1 max-w-md">
            <SearchBar />
          </div>
          <button
            className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
            onClick={handleFullScreen}
          >
            Capture screen
          </button>
        </header>
        <section className="flex-1 overflow-auto p-4">
          {selection.type === 'captures' ? (
            <CaptureGrid query={selection.query} />
          ) : (
            <ClipboardList />
          )}
        </section>
      </div>
    </main>
  );
}
