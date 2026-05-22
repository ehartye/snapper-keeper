import { useCallback, useRef, useState, type MutableRefObject } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type Konva from 'konva';

import { saveAnnotation } from '@snk/annotate';

import { useAnnotateStore } from './store';

interface Props {
  captureId: string;
  stageRef: MutableRefObject<Konva.Stage | null>;
}

type ButtonStatus = 'idle' | 'ok' | 'err';

function useFlash(): [ButtonStatus, (s: 'ok' | 'err') => void] {
  const [status, setStatus] = useState<ButtonStatus>('idle');
  const flash = useCallback((s: 'ok' | 'err') => {
    setStatus(s);
    window.setTimeout(() => setStatus('idle'), 1500);
  }, []);
  return [status, flash];
}

export function AnnotateTopBar({ captureId, stageRef }: Props) {
  const saving = useRef(false);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);
  const [saveStatus, flashSave] = useFlash();
  const [copyStatus, flashCopy] = useFlash();

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
      if (!png) throw new Error('nothing to save');
      await saveAnnotation(captureId, png);
      flashSave('ok');
    } catch (e) {
      console.error('save annotation failed', e);
      flashSave('err');
    } finally {
      saving.current = false;
    }
  }, [captureId, exportPng, flashSave]);

  const handleCopy = useCallback(async () => {
    const stage = stageRef.current;
    if (!stage) {
      flashCopy('err');
      return;
    }
    try {
      const blob = (await stage.toBlob({ pixelRatio: 1 / (stage.scaleX() || 1) })) as Blob | null;
      if (!blob) throw new Error('encode failed');
      await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
      flashCopy('ok');
    } catch (e) {
      console.error('copy failed', e);
      flashCopy('err');
    }
  }, [stageRef, flashCopy]);

  const handleDone = useCallback(async () => {
    useAnnotateStore.getState().reset();
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  const saveLabel =
    saveStatus === 'ok' ? 'saved ✓' : saveStatus === 'err' ? 'failed' : 'save';
  const copyLabel =
    copyStatus === 'ok' ? 'copied ✓' : copyStatus === 'err' ? 'failed' : 'copy';

  return (
    <div className="flex items-center gap-2 px-4 py-2.5 bg-bg-soft border-b border-border">
      <span className="font-display text-xs uppercase tracking-wider text-fg-muted flex-1">
        ✎ annotating · {captureId.slice(0, 8)}
      </span>
      <button
        onClick={handleCopy}
        className={`px-3 py-1 text-xs font-display uppercase tracking-wider rounded-lg transition-colors ${
          copyStatus === 'ok'
            ? 'bg-accent-3 text-bg'
            : copyStatus === 'err'
              ? 'bg-danger text-bg'
              : 'text-fg hover:bg-surface-2'
        }`}
      >
        {copyLabel}
      </button>
      <button
        onClick={handleSave}
        className={`px-3 py-1 text-xs font-display uppercase tracking-wider rounded-lg transition-colors ${
          saveStatus === 'ok'
            ? 'bg-accent-3 text-bg'
            : saveStatus === 'err'
              ? 'bg-danger text-bg'
              : 'bg-accent-2 text-bg hover:opacity-90'
        }`}
      >
        {saveLabel}
      </button>
      <button
        onClick={handleDone}
        className="px-3 py-1 text-xs font-display uppercase tracking-wider text-fg hover:bg-surface-2 rounded-lg"
      >
        done
      </button>
    </div>
  );
}
