import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { softDeleteCapture } from '@snk/library';

interface ToolbarPayload {
  captureId: string;
}

export function CaptureToolbar() {
  const [captureId, setCaptureId] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

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

  // Resize the window to fit the toolbar exactly. Re-runs whenever the layout
  // could change (mount, theme/font swap, content edits).
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const rect = el.getBoundingClientRect();
      // Add a small margin for the drop shadow (offset 4px right/down).
      const w = Math.ceil(rect.width) + 8;
      const h = Math.ceil(rect.height) + 8;
      getCurrentWindow().setSize(new LogicalSize(w, h)).catch(() => {});
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const dismiss = useCallback(async () => {
    const win = getCurrentWindow();
    await win.hide();
    setCaptureId(null);
  }, []);

  const handleAnnotate = useCallback(async () => {
    if (!captureId) {
      await dismiss();
      return;
    }
    const annotateWin = await WebviewWindow.getByLabel('annotate');
    if (annotateWin) {
      await annotateWin.emit('annotate:open', { captureId });
      await annotateWin.show();
      await annotateWin.setFocus();
    }
    await dismiss();
  }, [captureId, dismiss]);

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
    <div
      ref={rootRef}
      className="inline-flex items-center gap-1 bg-surface rounded-xl px-2 py-1.5 border-2 border-border shadow-[4px_4px_0_0_var(--accent-2)] font-display"
    >
      <button
        onClick={handleAnnotate}
        className="px-3 py-1 text-[11px] uppercase tracking-wider text-fg hover:bg-surface-2 rounded-lg"
        title="Annotate"
      >
        annotate
      </button>
      <button
        onClick={handleCopy}
        className="px-3 py-1 text-[11px] uppercase tracking-wider text-fg hover:bg-surface-2 rounded-lg"
      >
        copy
      </button>
      <button
        onClick={handleSave}
        className="px-3 py-1 text-[11px] uppercase tracking-wider text-accent-2 hover:bg-surface-2 rounded-lg"
      >
        save
      </button>
      <button
        onClick={handleDiscard}
        className="px-3 py-1 text-[11px] uppercase tracking-wider text-danger hover:bg-surface-2 rounded-lg"
      >
        discard
      </button>
    </div>
  );
}
