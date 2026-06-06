import { useEffect, useCallback, useRef, useState, type ChangeEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

import {
  listClipboardItems,
  pasteItem,
  toggleClipboardPin,
  clipboardStatus,
  CLIPBOARD_POPUP_SHOW_EVENT,
  CLIPBOARD_UNAVAILABLE_EVENT,
  CLIPBOARD_AVAILABLE_EVENT,
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
  const [offline, setOffline] = useState(false);

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

  // Track clipboard watcher availability so we can show an offline banner
  // instead of a silently-empty history. Seed from the current status, then
  // follow the watcher's transition events.
  useEffect(() => {
    let unlisten: Array<() => void> = [];
    clipboardStatus()
      .then((s) => setOffline(s ? !s.available : false))
      .catch(() => {});
    Promise.all([
      listen(CLIPBOARD_UNAVAILABLE_EVENT, () => setOffline(true)),
      listen(CLIPBOARD_AVAILABLE_EVENT, () => setOffline(false)),
    ])
      .then((fns) => {
        unlisten = fns;
      })
      .catch((e) => console.error('clipboard status listen failed', e));
    return () => unlisten.forEach((fn) => fn());
  }, []);

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

  // Use a window-level listener so navigation works regardless of which
  // element has focus inside the popup webview (input, body, list buttons).
  useEffect(() => {
    const handler = async (e: globalThis.KeyboardEvent) => {
      // Snapshot the latest state at the moment the key was pressed.
      const { items: liveItems, selectedIndex: liveIdx, filter: liveFilter } =
        useClipboardPopupStore.getState();

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
        const item = liveItems[liveIdx];
        if (item) await handlePaste(item.id);
        return;
      }
      // Plain 1-9 immediately pastes that row. Skip when a modifier is held
      // so OS shortcuts still work, and skip if the filter input is focused
      // and the user is editing — but only when the filter already has
      // content (so empty-filter quick-pick still works).
      if (
        !e.ctrlKey &&
        !e.altKey &&
        !e.metaKey &&
        /^[1-9]$/.test(e.key)
      ) {
        const isEditingFilter =
          document.activeElement === inputRef.current && liveFilter.length > 0;
        if (isEditingFilter) return;
        const idx = parseInt(e.key, 10) - 1;
        if (liveItems[idx]) {
          e.preventDefault();
          await handlePaste(liveItems[idx]!.id);
        }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
        const item = liveItems[liveIdx];
        if (item) {
          await toggleClipboardPin(item.id, !item.pinned);
          await loadItems(liveFilter);
        }
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [dismiss, handlePaste, moveSelection, loadItems]);

  return (
    <div className="flex flex-col h-full bg-surface border-2 border-border rounded-xl shadow-[6px_6px_0_0_var(--accent-2)] overflow-hidden">
      <div className="px-3 pt-3 pb-2 border-b border-border">
        {/* Drag handle — Tauri's data-tauri-drag-region attribute intercepts
            mousedown on this element so the user can pick up the popup and
            move it. The filter input below is a separate element and stays
            interactive. Each child also carries the attribute so dragging
            works whether the user grabs the grip glyph or the title text. */}
        <div
          data-tauri-drag-region
          className="font-display text-[10px] uppercase tracking-widest text-accent mb-2 px-1 flex items-center gap-2 cursor-move select-none"
        >
          <span
            data-tauri-drag-region
            aria-hidden
            className="text-[12px] leading-none text-fg-muted"
          >
            ⠿
          </span>
          <span data-tauri-drag-region>clipboard ✦</span>
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
      {offline && (
        <div className="px-3 py-1.5 bg-accent/10 text-accent text-[10px] font-display uppercase tracking-wider border-b border-border text-center">
          clipboard offline — retrying…
        </div>
      )}
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
