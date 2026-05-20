import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';

import { CAPTURE_FULL_SCREEN_EVENT, captureFullScreen } from '@snk/capture';

import { CaptureGrid } from './CaptureGrid';

export function LibraryWindow() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen(CAPTURE_FULL_SCREEN_EVENT, async () => {
      try {
        await captureFullScreen();
      } catch (e) {
        console.error('capture failed', e);
      }
      await queryClient.invalidateQueries({ queryKey: ['captures'] });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('listen failed', e));
    return () => unlisten?.();
  }, [queryClient]);

  return (
    <main className="h-full flex flex-col">
      <header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
        <h1 className="text-sm font-semibold">snapper-keeper</h1>
        <span className="text-xs text-slate-500">phase 1 · vertical slice</span>
        <div className="flex-1" />
        <button
          className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
          onClick={async () => {
            try {
              await captureFullScreen();
              await queryClient.invalidateQueries({ queryKey: ['captures'] });
            } catch (e) {
              console.error(e);
            }
          }}
        >
          Capture full screen
        </button>
      </header>
      <section className="flex-1 overflow-auto p-4">
        <CaptureGrid />
      </section>
    </main>
  );
}
