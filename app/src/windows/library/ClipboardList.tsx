import { useQuery, useQueryClient } from '@tanstack/react-query';

import { listClipboardItems, toggleClipboardPin } from '@snk/clipboard';
import type { ClipboardItem } from '@snk/clipboard';

import { formatTimeAgo } from '../../lib/formatTimeAgo';
import { queryKeys } from '../../lib/queryKeys';

function kindIcon(kind: ClipboardItem['kind']): string {
  switch (kind) {
    case 'text':
      return 'T';
    case 'image':
      return '🖼';
    default:
      return '?';
  }
}

export function ClipboardList() {
  const queryClient = useQueryClient();
  const { data, isLoading, error } = useQuery({
    queryKey: queryKeys.clipboard.list(),
    queryFn: () => listClipboardItems(),
  });

  const handleTogglePin = async (item: ClipboardItem) => {
    await toggleClipboardPin(item.id, !item.pinned);
    await queryClient.invalidateQueries({ queryKey: queryKeys.clipboard.list() });
  };

  if (isLoading) return <p className="text-fg-muted">Loading…</p>;
  if (error) return <p className="text-danger">Error: {String(error)}</p>;

  const items = data ?? [];
  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <div className="font-display text-2xl mb-2 holo-shimmer bg-clip-text text-transparent">
          nothing copied yet
        </div>
        <div className="text-sm text-fg-muted">Copy something to get started.</div>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {items.map((item) => (
        <div
          key={item.id}
          className="flex items-start gap-3 p-3 rounded-xl bg-surface border-2 border-border hover:-translate-y-0.5 hover:shadow-[4px_4px_0_0_var(--accent)] transition-all"
        >
          <span className="text-lg w-6 text-center shrink-0 font-display text-accent-2">
            {kindIcon(item.kind)}
          </span>
          <div className="flex-1 min-w-0">
            <div className="text-sm text-fg truncate">
              {item.text_content
                ? item.text_content.slice(0, 120)
                : item.kind === 'image'
                  ? '(image)'
                  : '(empty)'}
            </div>
            <div className="text-[10px] text-fg-muted mt-0.5">
              {item.source_app ?? 'unknown'} · {formatTimeAgo(item.created_at)}
            </div>
          </div>
          <button
            className={`text-[10px] uppercase tracking-wider font-display px-2 py-0.5 rounded ${
              item.pinned
                ? 'bg-accent text-bg'
                : 'text-fg-muted hover:text-fg'
            }`}
            onClick={() => handleTogglePin(item)}
            title={item.pinned ? 'Unpin' : 'Pin'}
          >
            {item.pinned ? '★ pinned' : 'pin'}
          </button>
        </div>
      ))}
    </div>
  );
}
