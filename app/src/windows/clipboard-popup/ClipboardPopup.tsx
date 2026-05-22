import { useEffect, useCallback, useRef, type ChangeEvent, type KeyboardEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

import {
  listClipboardItems,
  pasteItem,
  toggleClipboardPin,
  CLIPBOARD_POPUP_SHOW_EVENT,
} from '@snk/clipboard';

import { useClipboardPopupStore } from './store';
import { ClipboardPopupItem } from './ClipboardPopupItem';

export function ClipboardPopup() {
  const items = useClipboardPopupStore((s) => s.items);
  const filter = useClipboardPopupStore((s) => s.filter);
  const selectedIndex = useClipboardPopupStore((s) => s.selectedIndex);
  const setItems = useClipboardPopupStore((s) => s.setItems);
  const setFilter = useClipboardPopupStore((s) => s.setFilter);
  const moveSelection = useClipboardPopupStore((s) => s.moveSelection);
  const reset = useClipboardPopupStore((s) => s.reset);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadItems = useCallback(
    async (filterText?: string) => {
      try {
        const result = await listClipboardItems({
          limit: 50,
          filter: filterText || undefined,
        });
        setItems(result);
      } catch (e) {
        console.error('load clipboard items failed', e);
      }
    },
    [setItems],
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen(CLIPBOARD_POPUP_SHOW_EVENT, async () => {
      reset();
      await loadItems();
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
      inputRef.current?.focus();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('clipboard popup listen failed', e));
    return () => unlisten?.();
  }, [loadItems, reset]);

  const dismiss = useCallback(async () => {
    reset();
    const win = getCurrentWindow();
    await win.hide();
  }, [reset]);

  const handlePaste = useCallback(
    async (id: string) => {
      try {
        await dismiss();
        await pasteItem(id);
      } catch (e) {
        console.error('paste failed', e);
      }
    },
    [dismiss],
  );

  const handleFilterChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setFilter(val);
      loadItems(val);
    },
    [setFilter, loadItems],
  );

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        await dismiss();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        moveSelection(1);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        moveSelection(-1);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        const item = items[selectedIndex];
        if (item) {
          await handlePaste(item.id);
        }
        return;
      }
      const num = parseInt(e.key, 10);
      if (num >= 1 && num <= 9 && items[num - 1]) {
        e.preventDefault();
        await handlePaste(items[num - 1]!.id);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
        const item = items[selectedIndex];
        if (item) {
          await toggleClipboardPin(item.id, !item.pinned);
          await loadItems(filter);
        }
      }
    },
    [items, selectedIndex, dismiss, handlePaste, moveSelection, loadItems, filter],
  );

  return (
    <div
      className="flex flex-col h-full bg-surface border-2 border-border rounded-xl shadow-[6px_6px_0_0_var(--accent-2)] overflow-hidden"
      onKeyDown={handleKeyDown}
    >
      <div className="px-3 pt-3 pb-2 border-b border-border">
        <div className="font-display text-[10px] uppercase tracking-widest text-accent mb-2 px-1">
          clipboard ✦
        </div>
        <input
          ref={inputRef}
          type="text"
          value={filter}
          onChange={handleFilterChange}
          placeholder="filter…"
          className="w-full bg-bg-soft text-xs text-fg px-3 py-1.5 rounded-md border border-border outline-none focus:border-primary placeholder:text-fg-muted"
        />
      </div>
      <div className="flex-1 overflow-y-auto">
        {items.length === 0 ? (
          <div className="px-3 py-6 text-xs text-fg-muted text-center">
            nothing copied yet
          </div>
        ) : (
          items.map((item, i) => (
            <ClipboardPopupItem
              key={item.id}
              item={item}
              index={i}
              isSelected={i === selectedIndex}
              onSelect={handlePaste}
            />
          ))
        )}
      </div>
      <div className="px-3 py-2 border-t border-border text-[10px] text-fg-muted flex gap-3 font-display uppercase tracking-wider">
        <span>↑↓ nav</span>
        <span>↵ paste</span>
        <span>1-9</span>
        <span>⌘P pin</span>
        <span>esc</span>
      </div>
    </div>
  );
}
