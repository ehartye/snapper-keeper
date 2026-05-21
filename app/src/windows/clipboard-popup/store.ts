import { create } from 'zustand';

import type { ClipboardItem } from '@snk/clipboard';

interface ClipboardPopupState {
  items: ClipboardItem[];
  filter: string;
  selectedIndex: number;

  setItems: (items: ClipboardItem[]) => void;
  setFilter: (filter: string) => void;
  setSelectedIndex: (index: number) => void;
  moveSelection: (delta: number) => void;
  reset: () => void;
}

const initialState = {
  items: [] as ClipboardItem[],
  filter: '',
  selectedIndex: 0,
};

export const useClipboardPopupStore = create<ClipboardPopupState>((set, get) => ({
  ...initialState,

  setItems: (items) => set({ items, selectedIndex: 0 }),
  setFilter: (filter) => set({ filter, selectedIndex: 0 }),
  setSelectedIndex: (index) => set({ selectedIndex: index }),

  moveSelection: (delta) => {
    const { items, selectedIndex } = get();
    if (items.length === 0) return;
    const next = Math.max(0, Math.min(items.length - 1, selectedIndex + delta));
    if (next === selectedIndex) return;
    set({ selectedIndex: next });
  },

  reset: () => set(initialState),
}));
