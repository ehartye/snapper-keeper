import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { path } from '@tauri-apps/api';
import { useQuery } from '@tanstack/react-query';
import type Konva from 'konva';

import { getCapture } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { useAnnotateStore } from './store';
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
      <div className="h-full flex items-center justify-center bg-slate-950 text-slate-500 text-sm">
        Waiting for capture…
      </div>
    );
  }

  const src = captureAssetUrl(root.data, capture.data.file_path);

  return (
    <div className="h-full flex flex-col">
      <AnnotateTopBar captureId={captureId} stageRef={stageRef} />
      <div className="flex flex-1 overflow-hidden">
        <AnnotateToolbar />
        <AnnotateCanvas
          imageSrc={src}
          imageWidth={capture.data.width}
          imageHeight={capture.data.height}
        />
      </div>
    </div>
  );
}
