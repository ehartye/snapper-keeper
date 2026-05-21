import { useCallback, useRef, type RefObject } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type Konva from 'konva';

import { saveAnnotation } from '@snk/annotate';

import { useAnnotateStore } from './store';

interface Props {
  captureId: string;
  stageRef: RefObject<Konva.Stage | null>;
}

export function AnnotateTopBar({ captureId, stageRef }: Props) {
  const saving = useRef(false);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);

  const exportPng = useCallback(async (): Promise<number[] | null> => {
    const stage = stageRef.current;
    if (!stage) return null;

    let dataUrl: string;
    if (cropRegion) {
      dataUrl = stage.toDataURL({
        x: cropRegion.x,
        y: cropRegion.y,
        width: cropRegion.width,
        height: cropRegion.height,
        pixelRatio: 1 / (stage.scaleX() || 1),
      });
    } else {
      dataUrl = stage.toDataURL({ pixelRatio: 1 / (stage.scaleX() || 1) });
    }

    const base64 = dataUrl.split(',')[1];
    if (!base64) return null;
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return Array.from(bytes);
  }, [stageRef, cropRegion]);

  const handleSave = useCallback(async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      const png = await exportPng();
      if (png) {
        await saveAnnotation(captureId, png);
      }
    } catch (e) {
      console.error('save annotation failed', e);
    } finally {
      saving.current = false;
    }
  }, [captureId, exportPng]);

  const handleCopy = useCallback(async () => {
    const stage = stageRef.current;
    if (!stage) return;
    try {
      const blob = (await stage.toBlob({ pixelRatio: 1 / (stage.scaleX() || 1) })) as Blob | null;
      if (blob) {
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': blob }),
        ]);
      }
    } catch (e) {
      console.error('copy failed', e);
    }
  }, [stageRef]);

  const handleDone = useCallback(async () => {
    useAnnotateStore.getState().reset();
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-slate-900 border-b border-slate-700">
      <span className="text-xs text-slate-400 flex-1">
        Annotating capture {captureId.slice(0, 8)}…
      </span>
      <button
        onClick={handleCopy}
        className="px-3 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Copy
      </button>
      <button
        onClick={handleSave}
        className="px-3 py-1 text-xs text-blue-400 hover:bg-slate-700 rounded"
      >
        Save
      </button>
      <button
        onClick={handleDone}
        className="px-3 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Done
      </button>
    </div>
  );
}
