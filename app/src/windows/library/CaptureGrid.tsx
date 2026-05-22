import { useQuery, useQueryClient } from '@tanstack/react-query';
import { path } from '@tauri-apps/api';

import { listCaptures, purgeTrash } from '@snk/library';
import type { ListCapturesQuery } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { Thumbnail } from './Thumbnail';

interface Props {
  query?: ListCapturesQuery;
}

export function CaptureGrid({ query }: Props) {
  const queryClient = useQueryClient();
  const inTrash = query?.deleted_only === true;

  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });
  const captures = useQuery({
    queryKey: queryKeys.captures.list(query),
    queryFn: () => listCaptures(query),
  });

  if (root.isLoading || captures.isLoading) {
    return <p className="text-fg-muted">Loading…</p>;
  }
  if (root.error || captures.error) {
    return (
      <p className="text-danger">
        Error loading library: {String(root.error ?? captures.error)}
      </p>
    );
  }

  const rows = captures.data ?? [];

  const handleEmptyTrash = async () => {
    if (rows.length === 0) return;
    if (
      !window.confirm(
        `Permanently delete ${rows.length} capture${rows.length === 1 ? '' : 's'} from trash? This cannot be undone.`,
      )
    )
      return;
    try {
      await purgeTrash();
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (e) {
      console.error('purge trash failed', e);
    }
  };

  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <div className="font-display text-3xl mb-3 holo-shimmer bg-clip-text text-transparent">
          {inTrash ? 'trash is empty' : 'nothing yet!'}
        </div>
        {!inTrash && (
          <div className="text-sm text-fg-muted max-w-xs">
            Press <kbd className="font-display text-xs bg-surface border-2 border-border px-1.5 py-0.5 rounded mx-1">Ctrl+Shift+3</kbd> or use the tray menu to capture.
          </div>
        )}
      </div>
    );
  }

  return (
    <div>
      {inTrash && (
        <div className="flex items-center justify-between mb-4 bg-surface border-2 border-border rounded-xl px-4 py-2 shadow-[3px_3px_0_0_var(--danger)]">
          <div className="text-sm">
            <span className="font-display text-base">Trash</span>
            <span className="text-fg-muted ml-2">
              {rows.length} item{rows.length === 1 ? '' : 's'} · click an item to permanently delete
            </span>
          </div>
          <button
            onClick={handleEmptyTrash}
            className="font-display text-[11px] uppercase tracking-widest px-3 py-1.5 bg-danger text-bg border-2 border-border shadow-[2px_2px_0_0_var(--border)] hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[0_0_0_0_var(--border)] transition-transform"
          >
            Empty trash
          </button>
        </div>
      )}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {rows.map((c) => (
          <Thumbnail
            key={c.id}
            capture={c}
            src={captureAssetUrl(root.data!, c.file_path)}
            inTrash={inTrash}
          />
        ))}
      </div>
    </div>
  );
}
