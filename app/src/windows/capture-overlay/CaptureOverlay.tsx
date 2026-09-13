import { useCallback, useEffect, useRef, useState, type MouseEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { captureRegion, type ScreenPreview } from '@snk/capture';
import { selectionPixels, type Selection } from './selection';

export function CaptureOverlay() {
  const [preview, setPreview] = useState<ScreenPreview | null>(null);
  const [rect, setRect] = useState<Selection | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const image = useRef<HTMLImageElement>(null);
  const activeToken = useRef<string | null>(null);
  const loadedToken = useRef<string | null>(null);
  const drag = useRef<Selection | null>(null);

  const cancel = useCallback(async () => {
    activeToken.current = null;
    loadedToken.current = null;
    drag.current = null;
    setPreview(null);
    setRect(null);
    setReady(false);
    await getCurrentWindow().hide();
  }, []);

  const handleMouseDown = (e: MouseEvent) => {
    if (!preview || loadedToken.current !== preview.token) return;
    drag.current = { startX: e.clientX, startY: e.clientY, endX: e.clientX, endY: e.clientY };
    setRect(drag.current);
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!drag.current) return;
    drag.current = { ...drag.current, endX: e.clientX, endY: e.clientY };
    setRect(drag.current);
  };

  const handleMouseUp = async (e: MouseEvent) => {
    const selection = drag.current;
    drag.current = null;
    setRect(null);
    if (!selection || !preview || loadedToken.current !== preview.token || !image.current) return;
    const pixels = selectionPixels(
      { ...selection, endX: e.clientX, endY: e.clientY },
      image.current.getBoundingClientRect(),
      preview.width,
      preview.height,
    );
    if (!pixels) return;
    const token = preview.token;
    loadedToken.current = null;
    setReady(false);
    const win = getCurrentWindow();
    await win.hide();
    if (activeToken.current !== token) return;
    try {
      const capture = await captureRegion(token, pixels.x, pixels.y, pixels.w, pixels.h);
      // Saving remains job-owned, but a completed older save must not interrupt
      // a replacement preview with its toolbar. Recheck across every UI await.
      if (activeToken.current !== token) return;
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      if (activeToken.current !== token) return;
      const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
      if (activeToken.current !== token) return;
      if (toolbar) {
        await toolbar.emit('toolbar:show', { captureId: capture.id });
        if (activeToken.current !== token) return;
        await toolbar.show();
        if (activeToken.current !== token) return;
        await toolbar.setFocus();
      }
    } catch (e) {
      console.error('region capture failed', e);
      // An older request must not replace a newly-arrived preview with its error.
      if (activeToken.current !== token) return;
      setError(
        typeof e === 'object' && e !== null && 'kind' in e && e.kind === 'stale-preview'
          ? 'This preview expired. Press Esc and start a new region capture.'
          : 'Could not save this region. Press Esc and try again.',
      );
      await win.show();
      if (activeToken.current !== token) return;
      await win.setFocus();
    }
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<ScreenPreview>('overlay:preview', (event) => {
      activeToken.current = event.payload.token;
      loadedToken.current = null;
      drag.current = null;
      setReady(false);
      setRect(null);
      setError(null);
      setPreview(event.payload);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
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
      {preview && (
        <img
          key={preview.token}
          ref={image}
          src={`${convertFileSrc(preview.path)}?v=${preview.token}`}
          alt=""
          className="fixed inset-0 w-full h-full object-fill pointer-events-none"
          draggable={false}
          onLoad={() => {
            if (activeToken.current !== preview.token) return;
            loadedToken.current = preview.token;
            setReady(true);
          }}
          onError={() => {
            if (activeToken.current === preview.token)
              setError('Could not load this preview. Press Esc and try again.');
          }}
        />
      )}
      <div
        className="fixed inset-0 pointer-events-none"
        style={{ backgroundColor: 'rgba(0, 0, 0, 0.3)' }}
      />
      {rect && (
        <div
          className="absolute border-2 border-blue-400"
          style={{ ...selectionStyle, backgroundColor: 'rgba(59, 130, 246, 0.1)', zIndex: 10 }}
        />
      )}
      <div className="fixed top-4 left-1/2 -translate-x-1/2 text-white text-sm bg-black/60 px-3 py-1 rounded z-20">
        {error ??
          (ready ? 'Drag to select region | Esc to cancel' : 'Loading preview... | Esc to cancel')}
      </div>
    </div>
  );
}
