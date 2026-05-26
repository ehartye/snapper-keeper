import { useEffect, useRef } from 'react';
import { useInfiniteQuery, useQuery, useQueryClient } from '@tanstack/react-query';
import { path } from '@tauri-apps/api';

import { listCaptures, purgeTrash } from '@snk/library';
import type { Capture, ListCapturesQuery } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { formatShortcutForPlatform } from '../../lib/shortcuts';
import { Thumbnail } from './Thumbnail';

interface Props {
  query?: ListCapturesQuery;
}

const PAGE_SIZE = 60;

export function CaptureGrid({ query }: Props) {
  const queryClient = useQueryClient();
  const inTrash = query?.deleted_only === true;
  const loadMoreRef = useRef<HTMLDivElement>(null);

  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });

  const pages = useInfiniteQuery({
    queryKey: queryKeys.captures.list(query),
    queryFn: ({ pageParam }) =>
      listCaptures({
        ...(query ?? {}),
        limit: PAGE_SIZE,
        ...(pageParam !== undefined ? { before: pageParam } : {}),
      }),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (lastPage: Capture[]) => {
      // If we got a full page, there might be more. Cursor = oldest row's
      // created_at; the next request fetches rows created BEFORE that.
      if (lastPage.length < PAGE_SIZE) return undefined;
      return lastPage[lastPage.length - 1]?.created_at;
    },
  });

  const rows = pages.data?.pages.flat() ?? [];

  // Trigger fetchNextPage when the sentinel scrolls into view.
  useEffect(() => {
    const el = loadMoreRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (
          entries[0]?.isIntersecting &&
          pages.hasNextPage &&
          !pages.isFetchingNextPage
        ) {
          pages.fetchNextPage();
        }
      },
      { rootMargin: '300px' },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [pages, rows.length]);

  if (root.isLoading || (pages.isLoading && rows.length === 0)) {
    return <p className="text-fg-muted">Loading…</p>;
  }
  if (root.error || pages.error) {
    return (
      <p className="text-danger">
        Error loading library: {String(root.error ?? pages.error)}
      </p>
    );
  }

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
            Press <kbd className="font-display text-xs bg-surface border-2 border-border px-1.5 py-0.5 rounded mx-1">{formatShortcutForPlatform('CmdOrCtrl+Shift+3')}</kbd> or use the tray menu to capture.
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
              {rows.length}
              {pages.hasNextPage ? '+' : ''} item{rows.length === 1 ? '' : 's'}
              {' '}· click an item to permanently delete
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

      {/* Infinite-scroll sentinel — fetching the next page is triggered when
       * this scrolls within 300px of the viewport. */}
      <div ref={loadMoreRef} className="h-12 flex items-center justify-center mt-4">
        {pages.isFetchingNextPage ? (
          <span className="font-display text-xs uppercase tracking-wider text-fg-muted">
            loading more…
          </span>
        ) : pages.hasNextPage ? (
          <span className="text-fg-muted text-xs">scroll for more</span>
        ) : rows.length > PAGE_SIZE ? (
          <span className="text-fg-muted text-xs">— end —</span>
        ) : null}
      </div>
    </div>
  );
}
