import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

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

const SMART_SECTIONS: { label: string; icon: string; query: ListCapturesQuery }[] = [
  { label: 'All', icon: '◇', query: {} },
  { label: 'Today', icon: '✶', query: { since: startOfDay() } },
  { label: 'This Week', icon: '☀', query: { since: startOfWeek() } },
  { label: 'Pinned', icon: '★', query: { pinned_only: true } },
  { label: 'Trash', icon: '✕', query: { deleted_only: true } },
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

  const openSettings = async () => {
    const settings = await WebviewWindow.getByLabel('settings');
    if (settings) {
      await settings.show();
      await settings.setFocus();
    }
  };

  return (
    <aside className="w-60 shrink-0 border-r border-border flex flex-col overflow-y-auto bg-bg-soft">
      <nav className="p-3 space-y-1">
        {SMART_SECTIONS.map((s) => {
          const active = isActive(selection, s.label);
          return (
            <button
              key={s.label}
              className={`w-full text-left px-3 py-2 rounded-lg text-sm flex items-center gap-2 transition-colors ${
                active
                  ? 'bg-primary text-bg shadow-[3px_3px_0_0_var(--border)]'
                  : 'text-fg-muted hover:bg-surface hover:text-fg'
              }`}
              onClick={() =>
                onSelect({
                  type: 'captures',
                  label: s.label,
                  query:
                    s.label === 'Today'
                      ? { since: startOfDay() }
                      : s.label === 'This Week'
                        ? { since: startOfWeek() }
                        : s.query,
                })
              }
            >
              <span className="w-5 text-base leading-none">{s.icon}</span>
              <span>{s.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="squiggle-divider mx-3 my-2" />

      <div className="p-3">
        <div className="flex items-baseline justify-between mb-2 px-1">
          <div className="font-display text-[11px] uppercase tracking-wider">Tags</div>
          <button
            className="text-[10px] text-fg-muted hover:text-fg"
            onClick={() => setTagDialogOpen(true)}
          >
            manage
          </button>
        </div>
        {tags.length === 0 ? (
          <div className="text-xs text-fg-muted px-3">No tags yet</div>
        ) : (
          <nav className="space-y-1">
            {tags.map((tag) => {
              const active = isActive(selection, tag.name);
              return (
                <button
                  key={tag.id}
                  className={`w-full text-left px-3 py-1.5 rounded-lg text-sm flex items-center gap-2 transition-colors ${
                    active ? 'bg-surface text-fg' : 'text-fg-muted hover:bg-surface hover:text-fg'
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
                    className="w-3 h-3 rounded-full shrink-0 ring-2 ring-bg-soft"
                    style={{ backgroundColor: tag.color }}
                  />
                  {tag.name}
                </button>
              );
            })}
          </nav>
        )}
      </div>

      <div className="squiggle-divider mx-3 my-2" />

      <nav className="p-3">
        <button
          className={`w-full text-left px-3 py-2 rounded-lg text-sm flex items-center gap-2 transition-colors ${
            selection.type === 'clipboard'
              ? 'bg-accent-2 text-bg shadow-[3px_3px_0_0_var(--border)]'
              : 'text-fg-muted hover:bg-surface hover:text-fg'
          }`}
          onClick={() => onSelect({ type: 'clipboard' })}
        >
          <span className="w-5 text-base leading-none">⌘</span>
          <span>Clipboard History</span>
        </button>
      </nav>

      <div className="mt-auto">
        <div className="squiggle-divider mx-3 my-2" />
        <nav className="p-3">
          <button
            className="w-full text-left px-3 py-2 rounded-lg text-sm flex items-center gap-2 transition-colors text-fg-muted hover:bg-surface hover:text-fg"
            onClick={openSettings}
          >
            <span className="w-5 text-base leading-none">⚙</span>
            <span>Settings</span>
          </button>
        </nav>
      </div>
      <TagDialog open={tagDialogOpen} onClose={() => setTagDialogOpen(false)} />
    </aside>
  );
}
