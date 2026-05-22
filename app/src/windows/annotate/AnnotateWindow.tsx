import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { path } from '@tauri-apps/api';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useQuery } from '@tanstack/react-query';
import type Konva from 'konva';

import { getCapture } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { useAnnotateStore } from './store';
import { confirmDiscardIfDirty } from './dirtyGuard';
import { AnnotateToolbar } from './AnnotateToolbar';
import { AnnotateCanvas } from './AnnotateCanvas';
import { AnnotateTopBar } from './AnnotateTopBar';

interface AnnotatePayload {
  captureId: string;
}

export function AnnotateWindow() {
  const [captureId, setCaptureId] = useState<string | null>(null);
  const stageRef = useRef<Konva.Stage | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<AnnotatePayload>('annotate:open', (event) => {
      useAnnotateStore.getState().reset();
      setCaptureId(event.payload.captureId);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('annotate listen failed', e));
    return () => unlisten?.();
  }, []);

  // X button → hide instead of destroy, so the next time a thumbnail or the
  // capture toolbar opens this window the listeners are still live.
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        if (!confirmDiscardIfDirty()) return;
        useAnnotateStore.getState().reset();
        setCaptureId(null);
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('annotate close listener failed', e));
    return () => cleanup?.();
  }, []);

  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });

  const capture = useQuery({
    queryKey: queryKeys.captures.one(captureId ?? ''),
    queryFn: () => getCapture(captureId!),
    enabled: !!captureId,
  });

  if (!captureId || !capture.data || !root.data) {
    return (
      <div className="h-full flex items-center justify-center bg-bg text-fg-muted text-sm font-display uppercase tracking-wider">
        waiting for capture…
      </div>
    );
  }

  const src = captureAssetUrl(root.data, capture.data.file_path);

  return (
    <div className="h-full flex flex-col bg-bg text-fg">
      <AnnotateTopBar captureId={captureId} stageRef={stageRef} />
      <div className="flex flex-1 overflow-hidden">
        <AnnotateToolbar />
        <AnnotateCanvas
          imageSrc={src}
          imageWidth={capture.data.width}
          imageHeight={capture.data.height}
          stageRef={stageRef}
        />
      </div>
    </div>
  );
}
