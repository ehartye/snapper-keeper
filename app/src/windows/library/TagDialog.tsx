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
    if (editingId) {
      await updateTag(editingId, name.trim(), color);
    } else {
      await createTag(name.trim(), color);
    }
    await queryClient.invalidateQueries({ queryKey: queryKeys.tags.list() });
    setName('');
    setEditingId(null);
  };

  const handleDelete = async (id: string) => {
    await deleteTag(id);
    await queryClient.invalidateQueries({ queryKey: queryKeys.tags.list() });
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-slate-900 border border-slate-700 rounded-lg w-80 p-4">
        <div className="flex justify-between items-center mb-3">
          <h2 className="text-sm font-semibold text-slate-100">Manage Tags</h2>
          <button className="text-slate-400 hover:text-slate-200 text-xs" onClick={onClose}>
            Close
          </button>
        </div>

        <div className="space-y-1 mb-3 max-h-48 overflow-y-auto">
          {(tags ?? []).map((tag) => (
            <div
              key={tag.id}
              className="flex items-center gap-2 px-2 py-1 rounded hover:bg-slate-800 group"
            >
              <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: tag.color }} />
              <span className="text-sm text-slate-200 flex-1">{tag.name}</span>
              <button
                className="text-[10px] text-slate-500 hover:text-slate-300 opacity-0 group-hover:opacity-100"
                onClick={() => startEdit(tag)}
              >
                edit
              </button>
              <button
                className="text-[10px] text-red-500 hover:text-red-300 opacity-0 group-hover:opacity-100"
                onClick={() => handleDelete(tag.id)}
              >
                delete
              </button>
            </div>
          ))}
        </div>

        <div className="border-t border-slate-800 pt-3">
          <div className="text-[10px] text-slate-500 mb-1">{editingId ? 'Edit tag' : 'New tag'}</div>
          <input
            className="w-full bg-slate-800 text-slate-100 text-sm px-2 py-1 rounded border border-slate-700 mb-2"
            placeholder="Tag name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave();
            }}
          />
          <div className="flex gap-1 mb-2">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                className={`w-5 h-5 rounded-full border-2 ${
                  color === c ? 'border-white' : 'border-transparent'
                }`}
                style={{ backgroundColor: c }}
                onClick={() => setColor(c)}
              />
            ))}
          </div>
          <div className="flex gap-2">
            <button
              className="bg-slate-700 hover:bg-slate-600 text-slate-100 text-xs px-3 py-1 rounded flex-1"
              onClick={handleSave}
            >
              {editingId ? 'Update' : 'Create'}
            </button>
            {editingId && (
              <button
                className="text-xs text-slate-400 hover:text-slate-200 px-2"
                onClick={startCreate}
              >
                Cancel
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
