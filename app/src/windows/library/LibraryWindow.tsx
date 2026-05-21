import { useEffect, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
} from '@snk/capture';

import { CaptureGrid } from './CaptureGrid';

export function LibraryWindow() {
  const queryClient = useQueryClient();

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
    // Phase 2 MVP: for now, capture the first non-minimized window
    // that isn't our own app. A window picker UI is a future polish item.
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
    // 5-second countdown, then full-screen capture
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

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    const setup = async () => {
      unlisteners.push(await listen(CAPTURE_FULL_SCREEN_EVENT, handleFullScreen));
      unlisteners.push(await listen(CAPTURE_REGION_EVENT, handleRegion));
      unlisteners.push(await listen(CAPTURE_WINDOW_EVENT, handleWindow));
      unlisteners.push(await listen(CAPTURE_TIMED_EVENT, handleTimed));
    };
    setup().catch((e) => console.error('listen setup failed', e));
    return () => unlisteners.forEach((fn) => fn());
  }, [handleFullScreen, handleRegion, handleWindow, handleTimed]);

  return (
    <main className="h-full flex flex-col">
      <header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
        <h1 className="text-sm font-semibold">snapper-keeper</h1>
        <span className="text-xs text-slate-500">phase 3 · annotation editor</span>
        <div className="flex-1" />
        <button
          className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
          onClick={handleFullScreen}
        >
          Capture screen
        </button>
      </header>
      <section className="flex-1 overflow-auto p-4">
        <CaptureGrid />
      </section>
    </main>
  );
}
