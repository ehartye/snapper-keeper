import { useCallback, useEffect, useState, type MouseEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { captureRegion } from '@snk/capture';

interface Rect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

export function CaptureOverlay() {
  const [rect, setRect] = useState<Rect | null>(null);
  const [dragging, setDragging] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [monitorId, setMonitorId] = useState(0);
  const [monitorScaleFactor, setMonitorScaleFactor] = useState<number | null>(null);

  const cancel = useCallback(async () => {
    setPreviewSrc(null);
    setRect(null);
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  const handleMouseDown = useCallback((e: MouseEvent) => {
    setDragging(true);
    setRect({ startX: e.clientX, startY: e.clientY, endX: e.clientX, endY: e.clientY });
  }, []);

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!dragging || !rect) return;
      setRect((prev) => (prev ? { ...prev, endX: e.clientX, endY: e.clientY } : null));
    },
    [dragging, rect],
  );

  const handleMouseUp = useCallback(async () => {
    if (!rect) return;
    setDragging(false);

    const x = Math.min(rect.startX, rect.endX);
    const y = Math.min(rect.startY, rect.endY);
    const w = Math.abs(rect.endX - rect.startX);
    const h = Math.abs(rect.endY - rect.startY);

    setPreviewSrc(null);
    setRect(null);
    const win = getCurrentWindow();
    await win.hide();

    if (w < 5 || h < 5) return;

    await new Promise((r) => setTimeout(r, 150));

    try {
      const scaleFactor = monitorScaleFactor ?? (window.devicePixelRatio || 1);
      const capture = await captureRegion(
        monitorId,
        Math.round(x * scaleFactor),
        Math.round(y * scaleFactor),
        Math.round(w * scaleFactor),
        Math.round(h * scaleFactor),
      );
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
      if (toolbar) {
        await toolbar.emit('toolbar:show', { captureId: capture.id });
        await toolbar.show();
        await toolbar.setFocus();
      }
    } catch (e) {
      console.error('region capture failed', e);
    }
  }, [rect, monitorId, monitorScaleFactor]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ path: string; token: string; monitorId: number; scaleFactor: number }>(
      'overlay:preview',
      (event) => {
      // Cache-bust: the WebView caches by URL and the preview path is
      // stable across grabs, so ?v=<token> forces a fresh fetch. The
      // intermediate null clears the background between region snaps
      // so a slow refetch can't briefly show the previous frame.
      const url = `${convertFileSrc(event.payload.path)}?v=${event.payload.token}`;
      setMonitorId(event.payload.monitorId);
      setMonitorScaleFactor(event.payload.scaleFactor);
      setPreviewSrc(null);
      setPreviewSrc(url);
      },
    ).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  const selectionStyle = rect
    ? {
        left: Math.min(rect.startX, rect.endX),
        top: Math.min(rect.startY, rect.endY),
        width: Math.abs(rect.endX - rect.startX),
        height: Math.abs(rect.endY - rect.startY),
      }
    : undefined;

  return (
    <div
      className="fixed inset-0 cursor-crosshair select-none"
      style={{ backgroundColor: '#000' }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {/* Backdrop as <img>, not CSS background-image. The rest of the
          codebase consumes asset:// URLs via <img> (CaptureGrid →
          Thumbnail, AnnotateWindow → AnnotateCanvas). CSS
          background-image of asset:// URLs does not render in this
          Tauri 2 / WebView2 environment — `<img>` does. */}
      {previewSrc && (
        <img
          src={previewSrc}
          alt=""
          className="fixed inset-0 w-full h-full object-cover pointer-events-none"
          draggable={false}
        />
      )}
      {/* Dark tint over the screenshot */}
      <div className="fixed inset-0 pointer-events-none" style={{ backgroundColor: 'rgba(0, 0, 0, 0.3)' }} />
      {rect && dragging && (
        <div
          className="absolute border-2 border-blue-400"
          style={{
            ...selectionStyle,
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            zIndex: 10,
          }}
        />
      )}
      <div className="fixed top-4 left-1/2 -translate-x-1/2 text-white text-sm bg-black/60 px-3 py-1 rounded z-20">
        Drag to select region · Esc to cancel
      </div>
    </div>
  );
}
