import { useState, useCallback, useEffect, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { searchLibrary, type SearchResult } from '@snk/library';

export function SearchBar() {
  const [input, setInput] = useState('');
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const timerRef = useRef<ReturnType<typeof setTimeout>>();

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

  const handleClear = useCallback(() => {
    setInput('');
    setDebouncedQuery('');
  }, []);

  return (
    <div className="relative">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Search captures & clipboard..."
        className="w-full bg-slate-800 border border-slate-700 rounded px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-slate-500"
      />
      {input && (
        <button
          onClick={handleClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 text-xs"
        >
          Clear
        </button>
      )}
      {debouncedQuery && results && results.length > 0 && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-slate-800 border border-slate-700 rounded shadow-lg max-h-64 overflow-auto z-50">
          {results.map((result) => (
            <SearchResultRow key={resultKey(result)} result={result} />
          ))}
        </div>
      )}
      {debouncedQuery && results && results.length === 0 && !isLoading && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-slate-800 border border-slate-700 rounded shadow-lg p-3 z-50">
          <p className="text-slate-500 text-xs text-center">No results</p>
        </div>
      )}
    </div>
  );
}

function resultKey(result: SearchResult): string {
  return `${result.kind}-${result.id}`;
}

function SearchResultRow({ result }: { result: SearchResult }) {
  const icon = result.kind === 'capture' ? 'img' : 'txt';
  return (
    <div className="px-3 py-2 hover:bg-slate-700 cursor-pointer border-b border-slate-700 last:border-0">
      <div className="flex items-center gap-2">
        <span className="text-[10px] font-mono text-slate-500 uppercase w-6">{icon}</span>
        <span
          className="text-xs text-slate-300 truncate flex-1"
          dangerouslySetInnerHTML={{ __html: result.snippet }}
        />
      </div>
    </div>
  );
}
