import { useState, useRef, useEffect } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { Capture } from '@snk/library';
import { listTags, listCaptureTags, assignTag, removeTag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

interface Props {
  capture: Capture;
  src: string;
}

export function Thumbnail({ capture, src }: Props) {
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

  return (
    <>
      <div
        className="bg-slate-900 border border-slate-800 rounded-md overflow-hidden"
        onContextMenu={handleContextMenu}
      >
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
            {capture.width}x{capture.height}
            {capture.monitor ? ` · ${capture.monitor}` : ''}
            {capture.source_app ? ` · ${capture.source_app}` : ''}
          </div>
          {capture.annotated_path && (
            <div className="text-[10px] text-blue-400 truncate">annotated</div>
          )}
        </div>
      </div>

      {menuPos && (
        <div
          ref={menuRef}
          className="fixed bg-slate-800 border border-slate-700 rounded-md shadow-lg py-1 z-50 min-w-[140px]"
          style={{ left: menuPos.x, top: menuPos.y }}
        >
          <div className="text-[10px] text-slate-500 px-3 py-1">Tags</div>
          {(allTags.data ?? []).length === 0 ? (
            <div className="text-xs text-slate-500 px-3 py-1">No tags created</div>
          ) : (
            (allTags.data ?? []).map((tag) => (
              <button
                key={tag.id}
                className="w-full text-left px-3 py-1 text-sm text-slate-200 hover:bg-slate-700 flex items-center gap-2"
                onClick={() => handleToggleTag(tag.id, assignedIds.has(tag.id))}
              >
                <span
                  className="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ backgroundColor: tag.color }}
                />
                <span className="flex-1">{tag.name}</span>
                {assignedIds.has(tag.id) && <span className="text-green-400 text-xs">✓</span>}
              </button>
            ))
          )}
        </div>
      )}
    </>
  );
}
