import { useState, type MouseEvent as ReactMouseEvent } from 'react';
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

type CopyStatus = 'ok' | 'err';

export function ClipboardList() {
  const queryClient = useQueryClient();
  const { data, isLoading, error } = useQuery({
    queryKey: queryKeys.clipboard.list(),
    queryFn: () => listClipboardItems(),
  });
  // id → 'ok' | 'err' (cleared after 1.5s)
  const [flash, setFlash] = useState<Record<string, CopyStatus>>({});

  const handleTogglePin = async (item: ClipboardItem, e: ReactMouseEvent) => {
    e.stopPropagation();
    await toggleClipboardPin(item.id, !item.pinned);
    await queryClient.invalidateQueries({ queryKey: queryKeys.clipboard.list() });
  };

  const setFlashFor = (id: string, status: CopyStatus) => {
    setFlash((prev) => ({ ...prev, [id]: status }));
    window.setTimeout(() => {
      setFlash((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }, 1500);
  };

  const handleCopy = async (item: ClipboardItem) => {
    if (!item.text_content) {
      // Image / file rows can't be re-copied via navigator.clipboard (text-only)
      setFlashFor(item.id, 'err');
      return;
    }
    try {
      await navigator.clipboard.writeText(item.text_content);
      setFlashFor(item.id, 'ok');
    } catch (e) {
      console.error('copy failed', e);
      setFlashFor(item.id, 'err');
    }
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
      {items.map((item) => {
        const status = flash[item.id];
        return (
          <button
            key={item.id}
            onClick={() => handleCopy(item)}
            className={`w-full text-left flex items-start gap-3 p-3 rounded-xl border-2 transition-all ${
              status === 'ok'
                ? 'bg-accent-3 border-accent-3 text-bg'
                : status === 'err'
                  ? 'bg-danger border-danger text-bg'
                  : 'bg-surface border-border hover:-translate-y-0.5 hover:shadow-[4px_4px_0_0_var(--accent)]'
            }`}
          >
            <span
              className={`text-lg w-6 text-center shrink-0 font-display ${
                status ? 'text-bg' : 'text-accent-2'
              }`}
            >
              {kindIcon(item.kind)}
            </span>
            <div className="flex-1 min-w-0">
              <div className={`text-sm truncate ${status ? 'text-bg' : 'text-fg'}`}>
                {status === 'ok'
                  ? 'Copied ✓'
                  : status === 'err'
                    ? item.kind === 'image'
                      ? 'images can\'t be re-copied yet'
                      : 'copy failed'
                    : item.text_content
                      ? item.text_content.slice(0, 120)
                      : item.kind === 'image'
                        ? '(image)'
                        : '(empty)'}
              </div>
              <div
                className={`text-[10px] mt-0.5 ${
                  status ? 'text-bg/70' : 'text-fg-muted'
                }`}
              >
                {item.source_app ?? 'unknown'} · {formatTimeAgo(item.created_at)}
              </div>
            </div>
            <span
              className={`text-[10px] uppercase tracking-wider font-display px-2 py-0.5 rounded cursor-pointer ${
                item.pinned
                  ? 'bg-accent text-bg'
                  : 'text-fg-muted hover:text-fg'
              }`}
              onClick={(e) => handleTogglePin(item, e)}
              title={item.pinned ? 'Unpin' : 'Pin'}
              role="button"
            >
              {item.pinned ? '★ pinned' : 'pin'}
            </span>
          </button>
        );
      })}
    </div>
  );
}
