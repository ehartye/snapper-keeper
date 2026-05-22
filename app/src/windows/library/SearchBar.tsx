import { useState, useCallback, useEffect, useRef, type KeyboardEvent } from 'react';
import { useQuery } from '@tanstack/react-query';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { searchLibrary, type SearchResult } from '@snk/library';

interface Props {
  onSelectClipboard?: () => void;
}

export function SearchBar({ onSelectClipboard }: Props) {
  const [input, setInput] = useState('');
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const [open, setOpen] = useState(true);
  const [activeIndex, setActiveIndex] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout>>();
  const rowsRef = useRef<Array<HTMLButtonElement | null>>([]);

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setDebouncedQuery(input.trim());
    }, 250);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [input]);

  const { data: results, isLoading } = useQuery({
    queryKey: ['search', debouncedQuery],
    queryFn: () => searchLibrary(debouncedQuery, 20),
    enabled: debouncedQuery.length > 0,
  });

  // Reset highlight whenever the result list changes.
  useEffect(() => {
    setActiveIndex(0);
  }, [debouncedQuery]);

  // Keep the active row scrolled into view on arrow nav.
  useEffect(() => {
    rowsRef.current[activeIndex]?.scrollIntoView({ block: 'nearest' });
  }, [activeIndex]);

  const handleClear = useCallback(() => {
    setInput('');
    setDebouncedQuery('');
  }, []);

  const handleResultClick = useCallback(
    async (result: SearchResult) => {
      if (result.kind === 'capture') {
        const annotate = await WebviewWindow.getByLabel('annotate');
        if (annotate) {
          await annotate.show();
          await annotate.setFocus();
          await annotate.emit('annotate:open', { captureId: result.id });
        }
      } else if (result.kind === 'clipboard') {
        onSelectClipboard?.();
      }
      handleClear();
      setOpen(false);
    },
    [handleClear, onSelectClipboard],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (!open || !results || results.length === 0) {
        if (e.key === 'Escape' && input) {
          e.preventDefault();
          handleClear();
        }
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIndex((i) => Math.min(results.length - 1, i + 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const target = results[activeIndex];
        if (target) handleResultClick(target);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        if (input) handleClear();
        else setOpen(false);
      }
    },
    [open, results, activeIndex, input, handleClear, handleResultClick],
  );

  return (
    <div className="relative">
      <input
        type="text"
        value={input}
        onChange={(e) => {
          setInput(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        onKeyDown={handleKeyDown}
        placeholder="search captures & clipboard…"
        className="w-full bg-surface border-2 border-border rounded-lg px-3 py-1.5 text-sm text-fg placeholder:text-fg-muted focus:outline-none focus:border-primary"
      />
      {input && (
        <button
          onClick={handleClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 text-fg-muted hover:text-fg text-xs"
        >
          clear
        </button>
      )}
      {open && debouncedQuery && results && results.length > 0 && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-surface border-2 border-border rounded-lg shadow-[4px_4px_0_0_var(--border)] max-h-64 overflow-auto z-50">
          {results.map((result, i) => (
            <SearchResultRow
              key={resultKey(result)}
              result={result}
              active={i === activeIndex}
              onClick={() => handleResultClick(result)}
              onMouseEnter={() => setActiveIndex(i)}
              elementRef={(el) => {
                rowsRef.current[i] = el;
              }}
            />
          ))}
        </div>
      )}
      {open && debouncedQuery && results && results.length === 0 && !isLoading && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-surface border-2 border-border rounded-lg shadow-[4px_4px_0_0_var(--border)] p-3 z-50">
          <p className="text-fg-muted text-xs text-center">No results</p>
        </div>
      )}
    </div>
  );
}

function resultKey(result: SearchResult): string {
  return `${result.kind}-${result.id}`;
}

interface RowProps {
  result: SearchResult;
  active: boolean;
  onClick: () => void;
  onMouseEnter: () => void;
  elementRef: (el: HTMLButtonElement | null) => void;
}

function SearchResultRow({ result, active, onClick, onMouseEnter, elementRef }: RowProps) {
  const icon = result.kind === 'capture' ? 'img' : 'txt';
  return (
    <button
      ref={elementRef}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      className={`w-full text-left px-3 py-2 border-b border-border last:border-0 ${
        active ? 'bg-primary/10' : 'hover:bg-surface-2'
      }`}
    >
      <div className="flex items-center gap-2">
        <span
          className={`text-[10px] font-display uppercase w-8 ${active ? 'text-primary' : 'text-accent-2'}`}
        >
          {icon}
        </span>
        <span
          className="text-xs text-fg truncate flex-1"
          dangerouslySetInnerHTML={{ __html: result.snippet }}
        />
      </div>
    </button>
  );
}
