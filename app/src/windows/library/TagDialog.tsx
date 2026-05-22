import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { createTag, updateTag, deleteTag, listTags } from '@snk/library';
import type { Tag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

const PRESET_COLORS = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#3b82f6', '#8b5cf6', '#ec4899', '#64748b'];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TagDialog({ open, onClose }: Props) {
  const queryClient = useQueryClient();
  const { data: tags } = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
  });
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [color, setColor] = useState(PRESET_COLORS[0]!);

  if (!open) return null;

  const startEdit = (tag: Tag) => {
    setEditingId(tag.id);
    setName(tag.name);
    setColor(tag.color);
  };

  const startCreate = () => {
    setEditingId(null);
    setName('');
    setColor(PRESET_COLORS[0]!);
  };

  const handleSave = async () => {
    if (!name.trim()) return;
    try {
      if (editingId) {
        await updateTag(editingId, name.trim(), color);
      } else {
        await createTag(name.trim(), color);
      }
      await queryClient.invalidateQueries({ queryKey: queryKeys.tags.all() });
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
      setName('');
      setEditingId(null);
    } catch (e) {
      console.error('tag save failed', e);
    }
  };

  const handleDelete = async (id: string) => {
    const tag = (tags ?? []).find((t) => t.id === id);
    if (!window.confirm(`Delete tag "${tag?.name ?? id}"? This cannot be undone.`)) return;
    try {
      await deleteTag(id);
      await queryClient.invalidateQueries({ queryKey: queryKeys.tags.all() });
      await queryClient.invalidateQueries({ queryKey: queryKeys.captures.all() });
    } catch (e) {
      console.error('tag delete failed', e);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-surface border-2 border-border rounded-xl w-80 p-5 shadow-[6px_6px_0_0_var(--accent-2)]">
        <div className="flex justify-between items-center mb-4">
          <h2 className="font-display text-sm uppercase tracking-wider text-fg">Manage Tags</h2>
          <button className="text-fg-muted hover:text-fg text-xs" onClick={onClose}>
            close
          </button>
        </div>

        <div className="space-y-1 mb-4 max-h-48 overflow-y-auto">
          {(tags ?? []).map((tag) => (
            <div
              key={tag.id}
              className="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-surface-2 group"
            >
              <span
                className="w-3.5 h-3.5 rounded-full shrink-0 ring-2 ring-surface"
                style={{ backgroundColor: tag.color }}
              />
              <span className="text-sm text-fg flex-1">{tag.name}</span>
              <button
                className="text-[10px] text-fg-muted hover:text-fg opacity-0 group-hover:opacity-100"
                onClick={() => startEdit(tag)}
              >
                edit
              </button>
              <button
                className="text-[10px] text-danger hover:opacity-80 opacity-0 group-hover:opacity-100"
                onClick={() => handleDelete(tag.id)}
              >
                delete
              </button>
            </div>
          ))}
        </div>

        <div className="border-t border-border pt-4">
          <div className="font-display text-[10px] uppercase tracking-wider text-fg-muted mb-2">
            {editingId ? 'Edit tag' : 'New tag'}
          </div>
          <input
            className="w-full bg-bg-soft text-fg text-sm px-3 py-1.5 rounded-md border border-border mb-3 focus:outline-none focus:border-primary"
            placeholder="Tag name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave();
            }}
          />
          <div className="flex gap-1.5 mb-3">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                className={`w-6 h-6 rounded-full border-2 ${
                  color === c ? 'border-fg' : 'border-transparent'
                }`}
                style={{ backgroundColor: c }}
                onClick={() => setColor(c)}
              />
            ))}
          </div>
          <div className="flex gap-2">
            <button
              className="bg-primary text-bg text-xs font-display uppercase tracking-wider px-4 py-1.5 rounded-lg flex-1 hover:opacity-90"
              onClick={handleSave}
            >
              {editingId ? 'Update' : 'Create'}
            </button>
            {editingId && (
              <button
                className="text-xs text-fg-muted hover:text-fg px-2"
                onClick={startCreate}
              >
                cancel
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
