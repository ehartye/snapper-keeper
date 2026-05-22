import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';

import { useTheme, THEMES, DEFAULT_THEME } from './theme';

function wrap({ children }: { children: ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

describe('theme module', () => {
  it('exposes 14 themes across 7 families with light + dark each', () => {
    expect(THEMES).toHaveLength(14);
    const families = new Set(THEMES.map((t) => t.family));
    expect(families).toEqual(
      new Set(['holo', 'memphis', 'robotic', 'corporate', 'wabi-sabi', 'riso', 'constructivist']),
    );
    for (const fam of families) {
      const inFam = THEMES.filter((t) => t.family === fam);
      expect(inFam.map((t) => t.mode).sort()).toEqual(['dark', 'light']);
    }
  });

  it('familyOf handles multi-hyphen family slugs', async () => {
    const { familyOf } = await import('./theme');
    expect(familyOf('holo-dark')).toBe('holo');
    expect(familyOf('wabi-sabi-light')).toBe('wabi-sabi');
    expect(familyOf('wabi-sabi-dark')).toBe('wabi-sabi');
  });

  it('default theme is in the list', () => {
    expect(THEMES.find((t) => t.id === DEFAULT_THEME)).toBeDefined();
  });

  it('useTheme applies data-theme to <html> and broadcasts tray theme to Rust', async () => {
    vi.mocked(invoke).mockResolvedValue(null);
    const { result } = renderHook(() => useTheme(), { wrapper: wrap });

    await waitFor(() => {
      expect(document.documentElement.getAttribute('data-theme')).toBe(DEFAULT_THEME);
    });
    expect(result.current.theme).toBe(DEFAULT_THEME);
    // First applyTheme runs synchronously from the useEffect; the IPC call to
    // set_tray_theme is fired with the matching family slug.
    expect(invoke).toHaveBeenCalledWith('set_tray_theme', { family: 'holo' });
  });

  it('setTheme updates the data-theme attribute and persists', async () => {
    vi.mocked(invoke).mockResolvedValue(null);
    const { result } = renderHook(() => useTheme(), { wrapper: wrap });

    await waitFor(() => expect(result.current.theme).toBe(DEFAULT_THEME));

    await act(async () => {
      await result.current.setTheme('memphis-dark');
    });
    expect(document.documentElement.getAttribute('data-theme')).toBe('memphis-dark');
    expect(invoke).toHaveBeenCalledWith('set_tray_theme', { family: 'memphis' });
  });
});
