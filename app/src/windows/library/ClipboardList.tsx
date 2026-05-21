import { useQuery, useQueryClient } from '@tanstack/react-query';

import { listClipboardItems, toggleClipboardPin } from '@snk/clipboard';
import type { ClipboardItem } from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';

function formatTimeAgo(ms: number): string {
  const seconds = Math.floor((Date.now() - ms) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

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

  if (isLoading) return <p className="text-slate-500">Loading…</p>;
  if (error) return <p className="text-red-400">Error: {String(error)}</p>;

  const items = data ?? [];
  if (items.length === 0) {
    return (
      <div className="text-slate-500 text-sm">
        No clipboard items yet. Copy something to get started.
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {items.map((item) => (
        <div
          key={item.id}
          className="flex items-start gap-3 p-3 rounded-md bg-slate-900 border border-slate-800 hover:border-slate-700"
        >
          <span className="text-lg w-6 text-center shrink-0">{kindIcon(item.kind)}</span>
          <div className="flex-1 min-w-0">
            <div className="text-sm text-slate-200 truncate">
              {item.text_content
                ? item.text_content.slice(0, 120)
                : item.kind === 'image'
                  ? '(image)'
                  : '(empty)'}
            </div>
            <div className="text-[10px] text-slate-500 mt-0.5">
              {item.source_app ?? 'unknown'} · {formatTimeAgo(item.created_at)}
            </div>
          </div>
          <button
            className={`text-xs px-1.5 py-0.5 rounded ${
              item.pinned
                ? 'bg-amber-900 text-amber-300'
                : 'text-slate-500 hover:text-slate-300'
            }`}
            onClick={() => handleTogglePin(item)}
            title={item.pinned ? 'Unpin' : 'Pin'}
          >
            {item.pinned ? 'pinned' : 'pin'}
          </button>
        </div>
      ))}
    </div>
  );
}
