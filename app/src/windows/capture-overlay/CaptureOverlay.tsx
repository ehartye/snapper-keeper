import { useCallback, useEffect, useState, type MouseEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
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

  const cancel = useCallback(async () => {
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

    const win = getCurrentWindow();
    await win.hide();

    if (w < 5 || h < 5) return;

    try {
      const scaleFactor = window.devicePixelRatio || 1;
      await captureRegion(
        0,
        Math.round(x * scaleFactor),
        Math.round(y * scaleFactor),
        Math.round(w * scaleFactor),
        Math.round(h * scaleFactor),
      );
    } catch (e) {
      console.error('region capture failed', e);
    }
  }, [rect]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel]);

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
      style={{ backgroundColor: 'rgba(0, 0, 0, 0.3)' }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {rect && dragging && (
        <div
          className="absolute border-2 border-blue-400"
          style={{
            ...selectionStyle,
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
          }}
        />
      )}
      <div className="fixed top-4 left-1/2 -translate-x-1/2 text-white text-sm bg-black/60 px-3 py-1 rounded">
        Drag to select region · Esc to cancel
      </div>
    </div>
  );
}
