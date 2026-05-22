import { useState, useRef, useEffect } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import type { Capture } from '@snk/library';
import {
  listTags,
  listCaptureTags,
  assignTag,
  removeTag,
  softDeleteCapture,
  hardDeleteCapture,
  setCapturePinned,
} from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

interface Props {
  capture: Capture;
  src: string;
  inTrash?: boolean;
}

export function Thumbnail({ capture, src, inTrash = false }: Props) {
  const [loaded, setLoaded] = useState(false);
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const allTags = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
    enabled: menuPos !== null,
  });

  const captureTags = useQuery({
    queryKey: queryKeys.tags.forCapture(capture.id),
    queryFn: () => listCaptureTags(capture.id),
    enabled: menuPos !== null,
  });

  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuPos(null);
      }
    };
    if (menuPos) document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [menuPos]);

  const handleContextMenu = (e: ReactMouseEvent) => {
    e.preventDefault();
    setMenuPos({ x: e.clientX, y: e.clientY });
  };

  const handleTogglePin = async (e: ReactMouseEvent) => {
    e.stopPropagation();
    try {
      await setCapturePinned(capture.id, !capture.pinned);
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (err) {
      console.error('pin toggle failed', err);
    }
  };

  const handleTogglePinFromMenu = async () => {
    try {
      await setCapturePinned(capture.id, !capture.pinned);
      setMenuPos(null);
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (err) {
      console.error('pin toggle failed', err);
    }
  };

  const handleToggleTag = async (tagId: string, assigned: boolean) => {
    try {
      if (assigned) {
        await removeTag(capture.id, tagId);
      } else {
        await assignTag(capture.id, tagId);
      }
      await queryClient.invalidateQueries({ queryKey: queryKeys.tags.forCapture(capture.id) });
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (e) {
      console.error('tag toggle failed', e);
    }
  };

  const assignedIds = new Set((captureTags.data ?? []).map((t) => t.id));

  const handleClick = async () => {
    if (inTrash) {
      if (!window.confirm('Permanently delete this capture? This cannot be undone.')) return;
      try {
        await hardDeleteCapture(capture.id);
        await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
      } catch (err) {
        console.error('hard delete failed', err);
      }
      return;
    }
    const annotate = await WebviewWindow.getByLabel('annotate');
    if (annotate) {
      // Show first so the webview is alive when the event arrives, then emit.
      await annotate.show();
      await annotate.setFocus();
      await annotate.emit('annotate:open', { captureId: capture.id });
    }
  };

  const handleDelete = async (e: ReactMouseEvent) => {
    e.stopPropagation();
    if (inTrash) {
      if (!window.confirm('Permanently delete this capture? This cannot be undone.')) return;
      try {
        console.log('[trash] hard-deleting', capture.id);
        await hardDeleteCapture(capture.id);
        console.log('[trash] hard-deleted', capture.id);
        await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
      } catch (err) {
        console.error('hard delete failed:', err);
        const detail =
          typeof err === 'string'
            ? err
            : err instanceof Error
              ? err.message
              : JSON.stringify(err, null, 2);
        window.alert(`Delete failed: ${detail}`);
      }
      return;
    }
    try {
      await softDeleteCapture(capture.id);
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (err) {
      console.error('delete failed', err);
    }
  };

  return (
    <>
      <div
        className="bg-surface border-2 border-border rounded-xl overflow-hidden cursor-pointer hover:-translate-y-0.5 hover:shadow-[5px_5px_0_0_var(--accent-2)] transition-all duration-150 group relative"
        onContextMenu={handleContextMenu}
        onClick={handleClick}
      >
        <div className="relative aspect-video bg-bg-soft overflow-hidden">
          <img
            src={src}
            alt={`Capture ${capture.id}`}
            onLoad={() => setLoaded(true)}
            className={`w-full h-full object-cover transition-opacity ${
              loaded ? 'opacity-100' : 'opacity-0'
            }`}
          />
          {!inTrash && (
            <button
              className={`absolute top-2 left-2 w-7 h-7 rounded-full text-bg text-base border-2 border-border flex items-center justify-center transition-all shadow-[2px_2px_0_0_var(--border)] ${
                capture.pinned
                  ? 'bg-accent opacity-100'
                  : 'bg-surface-2 text-fg opacity-0 group-hover:opacity-100'
              }`}
              onClick={handleTogglePin}
              title={capture.pinned ? 'Unpin' : 'Pin'}
            >
              ★
            </button>
          )}
          <button
            className="absolute top-2 right-2 w-7 h-7 rounded-full bg-danger text-bg text-base font-bold border-2 border-border flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity shadow-[2px_2px_0_0_var(--border)] hover:rotate-90"
            onClick={handleDelete}
            title="Delete"
          >
            ×
          </button>
        </div>
        <div className="px-3 py-2 border-t-2 border-border">
          <div className="text-xs text-fg truncate font-medium">
            {new Date(capture.created_at).toLocaleTimeString()}
          </div>
          <div className="text-[10px] text-fg-muted truncate">
            {capture.width}×{capture.height}
            {capture.monitor ? ` · ${capture.monitor}` : ''}
            {capture.source_app ? ` · ${capture.source_app}` : ''}
          </div>
          {capture.annotated_path && (
            <div className="inline-block mt-1 text-[9px] uppercase tracking-wider font-display text-bg bg-accent-2 px-1.5 py-0.5 rounded">
              annotated
            </div>
          )}
        </div>
      </div>

      {menuPos && (
        <div
          ref={menuRef}
          className="fixed bg-surface border-2 border-border rounded-lg shadow-[4px_4px_0_0_var(--border)] py-1 z-50 min-w-[160px]"
          style={{ left: menuPos.x, top: menuPos.y }}
        >
          {!inTrash && (
            <>
              <button
                className="w-full text-left px-3 py-1.5 text-sm text-fg hover:bg-surface-2 flex items-center gap-2"
                onClick={handleTogglePinFromMenu}
              >
                <span className={capture.pinned ? 'text-accent' : 'text-fg-muted'}>★</span>
                <span>{capture.pinned ? 'Unpin' : 'Pin'}</span>
              </button>
              <div className="border-t border-border my-1" />
            </>
          )}
          <div className="text-[10px] font-display uppercase tracking-wider text-fg-muted px-3 py-1">Tags</div>
          {(allTags.data ?? []).length === 0 ? (
            <div className="text-xs text-fg-muted px-3 py-1">No tags created</div>
          ) : (
            (allTags.data ?? []).map((tag) => (
              <button
                key={tag.id}
                className="w-full text-left px-3 py-1.5 text-sm text-fg hover:bg-surface-2 flex items-center gap-2"
                onClick={() => handleToggleTag(tag.id, assignedIds.has(tag.id))}
              >
                <span
                  className="w-3 h-3 rounded-full shrink-0 ring-2 ring-surface"
                  style={{ backgroundColor: tag.color }}
                />
                <span className="flex-1">{tag.name}</span>
                {assignedIds.has(tag.id) && <span className="text-accent-3 text-xs">✓</span>}
              </button>
            ))
          )}
        </div>
      )}
    </>
  );
}
