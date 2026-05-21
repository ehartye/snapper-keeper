import { useState } from 'react';

import type { Capture } from '@snk/library';

interface Props {
  capture: Capture;
  src: string;
}

export function Thumbnail({ capture, src }: Props) {
  const [loaded, setLoaded] = useState(false);
  return (
    <div className="bg-slate-900 border border-slate-800 rounded-md overflow-hidden">
      <div className="relative aspect-video bg-slate-950">
        <img
          src={src}
          alt={`Capture ${capture.id}`}
          onLoad={() => setLoaded(true)}
          className={`w-full h-full object-cover transition-opacity ${
            loaded ? 'opacity-100' : 'opacity-0'
          }`}
        />
      </div>
      <div className="px-2 py-1.5">
        <div className="text-xs text-slate-200 truncate">
          {new Date(capture.created_at).toLocaleTimeString()}
        </div>
        <div className="text-[10px] text-slate-500 truncate">
          {capture.width}×{capture.height}
          {capture.monitor ? ` · ${capture.monitor}` : ''}
          {capture.source_app ? ` · ${capture.source_app}` : ''}
        </div>
      </div>
    </div>
  );
}
