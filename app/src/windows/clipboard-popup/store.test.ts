import { describe, it, expect, beforeEach } from 'vitest';

import { useClipboardPopupStore } from './store';

const item = (i: number) => ({
  id: `cb-${i}`,
  kind: 'text' as const,
  text_content: `t${i}`,
  image_path: null,
  content_hash: `${i}`,
  source_app: null,
  pinned: false,
  created_at: 0,
});

describe('useClipboardPopupStore', () => {
  beforeEach(() => useClipboardPopupStore.getState().reset());

  it('initialises empty', () => {
    const s = useClipboardPopupStore.getState();
    expect(s.items).toEqual([]);
    expect(s.filter).toBe('');
    expect(s.selectedIndex).toBe(0);
  });

  it('setItems replaces the list and snaps selection to 0', () => {
    const s = useClipboardPopupStore.getState();
    s.setItems([item(1), item(2), item(3)]);
    s.setSelectedIndex(2);
    s.setItems([item(99)]);
    expect(useClipboardPopupStore.getState().items).toHaveLength(1);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(0);
  });

  it('setFilter resets selectedIndex too', () => {
    const s = useClipboardPopupStore.getState();
    s.setItems([item(1), item(2)]);
    s.setSelectedIndex(1);
    s.setFilter('foo');
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(0);
  });

  it('moveSelection clamps to list bounds', () => {
    const s = useClipboardPopupStore.getState();
    s.setItems([item(1), item(2), item(3)]);

    s.moveSelection(2);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(2);
    s.moveSelection(5);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(2);
    s.moveSelection(-1);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(1);
    s.moveSelection(-10);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(0);
  });

  it('moveSelection is a no-op when list is empty', () => {
    const before = useClipboardPopupStore.getState().selectedIndex;
    useClipboardPopupStore.getState().moveSelection(1);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(before);
  });

  it('reset wipes everything', () => {
    const s = useClipboardPopupStore.getState();
    s.setItems([item(1)]);
    s.setFilter('hi');
    s.setSelectedIndex(0);
    s.reset();
    const after = useClipboardPopupStore.getState();
    expect(after.items).toEqual([]);
    expect(after.filter).toBe('');
  });
});
