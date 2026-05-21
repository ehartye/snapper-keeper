import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { listTags } from '@snk/library';
import type { ListCapturesQuery, Tag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';
import { TagDialog } from './TagDialog';

export type SidebarSelection =
  | { type: 'captures'; label: string; query: ListCapturesQuery }
  | { type: 'clipboard' };

function startOfDay(): number {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function startOfWeek(): number {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() - d.getDay());
  return d.getTime();
}

const SMART_SECTIONS: { label: string; query: ListCapturesQuery }[] = [
  { label: 'All', query: {} },
  { label: 'Today', query: { since: startOfDay() } },
  { label: 'This Week', query: { since: startOfWeek() } },
  { label: 'Pinned', query: { pinned_only: true } },
  { label: 'Trash', query: { deleted_only: true } },
];

interface Props {
  selection: SidebarSelection;
  onSelect: (s: SidebarSelection) => void;
}

function isActive(selection: SidebarSelection, label: string): boolean {
  if (selection.type === 'clipboard') return label === 'Clipboard History';
  return selection.label === label;
}

export function Sidebar({ selection, onSelect }: Props) {
  const tagsQuery = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
  });

  const tags: Tag[] = tagsQuery.data ?? [];
  const [tagDialogOpen, setTagDialogOpen] = useState(false);

  return (
    <aside className="w-56 shrink-0 border-r border-slate-800 flex flex-col overflow-y-auto">
      <nav className="p-2 space-y-0.5">
        {SMART_SECTIONS.map((s) => (
          <button
            key={s.label}
            className={`w-full text-left px-3 py-1.5 rounded text-sm ${
              isActive(selection, s.label)
                ? 'bg-slate-700 text-slate-100'
                : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
            }`}
            onClick={() => onSelect({ type: 'captures', label: s.label, query: s.label === 'Today' ? { since: startOfDay() } : s.label === 'This Week' ? { since: startOfWeek() } : s.query })}
          >
            {s.label}
          </button>
        ))}
      </nav>

      <div className="border-t border-slate-800 mx-2 my-1" />

      <div className="p-2">
        <div className="text-[10px] uppercase tracking-wider text-slate-500 px-3 mb-1">Tags</div>
        <button
          className="text-[10px] text-slate-500 hover:text-slate-300 px-3 mb-1"
          onClick={() => setTagDialogOpen(true)}
        >
          Manage tags
        </button>
        {tags.length === 0 ? (
          <div className="text-xs text-slate-600 px-3">No tags yet</div>
        ) : (
          <nav className="space-y-0.5">
            {tags.map((tag) => (
              <button
                key={tag.id}
                className={`w-full text-left px-3 py-1.5 rounded text-sm flex items-center gap-2 ${
                  isActive(selection, tag.name)
                    ? 'bg-slate-700 text-slate-100'
                    : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
                }`}
                onClick={() =>
                  onSelect({
                    type: 'captures',
                    label: tag.name,
                    query: { tag_id: tag.id },
                  })
                }
              >
                <span
                  className="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ backgroundColor: tag.color }}
                />
                {tag.name}
              </button>
            ))}
          </nav>
        )}
      </div>

      <div className="border-t border-slate-800 mx-2 my-1" />

      <nav className="p-2">
        <button
          className={`w-full text-left px-3 py-1.5 rounded text-sm ${
            selection.type === 'clipboard'
              ? 'bg-slate-700 text-slate-100'
              : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
          }`}
          onClick={() => onSelect({ type: 'clipboard' })}
        >
          Clipboard History
        </button>
      </nav>
      <TagDialog open={tagDialogOpen} onClose={() => setTagDialogOpen(false)} />
    </aside>
  );
}
