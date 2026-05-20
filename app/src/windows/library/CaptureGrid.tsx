import { useQuery } from '@tanstack/react-query';
import { path } from '@tauri-apps/api';

import { listCaptures } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { Thumbnail } from './Thumbnail';

export function CaptureGrid() {
  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });
  const captures = useQuery({
    queryKey: queryKeys.captures.list(),
    queryFn: () => listCaptures(),
  });

  if (root.isLoading || captures.isLoading) {
    return <p className="text-slate-500">Loading…</p>;
  }
  if (root.error || captures.error) {
    return (
      <p className="text-red-400">
        Error loading library: {String(root.error ?? captures.error)}
      </p>
    );
  }

  const rows = captures.data ?? [];
  if (rows.length === 0) {
    return (
      <div className="text-slate-500 text-sm">
        No captures yet. Press the hotkey or use the tray menu.
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
      {rows.map((c) => (
        <Thumbnail key={c.id} capture={c} src={captureAssetUrl(root.data!, c.file_path)} />
      ))}
    </div>
  );
}
