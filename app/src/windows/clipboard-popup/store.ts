import { create } from 'zustand';

import type { ClipboardItem, SourceApp } from '@snk/clipboard';

interface ClipboardPopupState {
  items: ClipboardItem[];
  filter: string;
  selectedIndex: number;
  targetApp: SourceApp | null;

  setItems: (items: ClipboardItem[]) => void;
  setFilter: (filter: string) => void;
  setSelectedIndex: (index: number) => void;
  setTargetApp: (app: SourceApp | null) => void;
  moveSelection: (delta: number) => void;
  reset: () => void;
}

const initialState = {
  items: [] as ClipboardItem[],
  filter: '',
  selectedIndex: 0,
  targetApp: null as SourceApp | null,
};

export const useClipboardPopupStore = create<ClipboardPopupState>((set, get) => ({
  ...initialState,

  setItems: (items) => set({ items, selectedIndex: 0 }),
  setFilter: (filter) => set({ filter, selectedIndex: 0 }),
  setSelectedIndex: (index) => set({ selectedIndex: index }),
  setTargetApp: (targetApp) => set({ targetApp }),

  moveSelection: (delta) => {
    const { items, selectedIndex } = get();
    if (items.length === 0) return;
    const next = Math.max(0, Math.min(items.length - 1, selectedIndex + delta));
    if (next === selectedIndex) return;
    set({ selectedIndex: next });
  },

  reset: () => set(initialState),
}));
