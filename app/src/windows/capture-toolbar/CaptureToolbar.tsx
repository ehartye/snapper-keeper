import { useCallback, useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { softDeleteCapture } from '@snk/library';

interface ToolbarPayload {
  captureId: string;
}

export function CaptureToolbar() {
  const [captureId, setCaptureId] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ToolbarPayload>('toolbar:show', (event) => {
      setCaptureId(event.payload.captureId);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('toolbar listen failed', e));
    return () => unlisten?.();
  }, []);

  const dismiss = useCallback(async () => {
    const win = getCurrentWindow();
    await win.hide();
    setCaptureId(null);
  }, []);

  const handleAnnotate = useCallback(async () => {
    await dismiss();
  }, [dismiss]);

  const handleCopy = useCallback(async () => {
    await dismiss();
  }, [dismiss]);

  const handleSave = useCallback(async () => {
    await dismiss();
  }, [dismiss]);

  const handleDiscard = useCallback(async () => {
    if (captureId) {
      try {
        await softDeleteCapture(captureId);
      } catch (e) {
        console.error('discard failed', e);
      }
    }
    await dismiss();
  }, [captureId, dismiss]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') dismiss();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dismiss]);

  return (
    <div className="flex items-center gap-1 bg-slate-900/95 rounded-lg px-2 py-1 border border-slate-700 shadow-lg">
      <button
        onClick={handleAnnotate}
        className="px-2 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
        title="Annotate (phase 3)"
      >
        Annotate
      </button>
      <button
        onClick={handleCopy}
        className="px-2 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Copy
      </button>
      <button
        onClick={handleSave}
        className="px-2 py-1 text-xs text-blue-400 hover:bg-slate-700 rounded"
      >
        Save
      </button>
      <button
        onClick={handleDiscard}
        className="px-2 py-1 text-xs text-red-400 hover:bg-slate-700 rounded"
      >
        Discard
      </button>
    </div>
  );
}
