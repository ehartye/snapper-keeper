import { useEffect, useCallback, useRef, type ChangeEvent, type KeyboardEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

import {
  listClipboardItems,
  pasteItem,
  toggleClipboardPin,
  CLIPBOARD_HISTORY_EVENT,
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
    listen(CLIPBOARD_HISTORY_EVENT, async () => {
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
      className="flex flex-col h-full bg-slate-900/95 border border-slate-700 rounded-lg shadow-2xl"
      onKeyDown={handleKeyDown}
    >
      <div className="px-3 pt-3 pb-2">
        <input
          ref={inputRef}
          type="text"
          value={filter}
          onChange={handleFilterChange}
          placeholder="Type to filter..."
          className="w-full bg-slate-800 text-xs text-slate-200 px-3 py-1.5 rounded border border-slate-600 outline-none focus:border-blue-500"
        />
      </div>
      <div className="flex-1 overflow-y-auto">
        {items.length === 0 ? (
          <div className="px-3 py-4 text-xs text-slate-500 text-center">
            No clipboard items
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
      <div className="px-3 py-1.5 border-t border-slate-700 text-[10px] text-slate-500 flex gap-3">
        <span>↑↓ nav</span>
        <span>Enter paste</span>
        <span>1-9 jump</span>
        <span>Ctrl+P pin</span>
        <span>Esc close</span>
      </div>
    </div>
  );
}
