import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from './queryKeys';

export type ThemeFamily = 'holo' | 'memphis' | 'robotic' | 'corporate';

export type ThemeId =
  | 'holo-light'
  | 'holo-dark'
  | 'memphis-light'
  | 'memphis-dark'
  | 'robotic-light'
  | 'robotic-dark'
  | 'corporate-light'
  | 'corporate-dark';

export const DEFAULT_THEME: ThemeId = 'holo-dark';

export const THEMES: {
  id: ThemeId;
  label: string;
  family: ThemeFamily;
  mode: 'light' | 'dark';
}[] = [
  { id: 'holo-light', label: 'Holographic Dreamcore — Light', family: 'holo', mode: 'light' },
  { id: 'holo-dark', label: 'Holographic Dreamcore — Dark', family: 'holo', mode: 'dark' },
  { id: 'memphis-light', label: 'Memphis Machine — Light', family: 'memphis', mode: 'light' },
  { id: 'memphis-dark', label: 'Memphis Machine — Dark', family: 'memphis', mode: 'dark' },
  { id: 'robotic-light', label: 'Mr Robotic — Light', family: 'robotic', mode: 'light' },
  { id: 'robotic-dark', label: 'Mr Robotic — Dark', family: 'robotic', mode: 'dark' },
  { id: 'corporate-light', label: 'Corporate Overlord — Light', family: 'corporate', mode: 'light' },
  { id: 'corporate-dark', label: 'Corporate Overlord — Dark', family: 'corporate', mode: 'dark' },
];

const SETTING_KEY = 'theme';

function applyTheme(theme: ThemeId) {
  document.documentElement.setAttribute('data-theme', theme);
  // Tray + window icons are global; only the library window broadcasts the
  // change to Rust so we don't spam the IPC with one call per window.
  if (getCurrentWindow().label !== 'library') return;
  const family = theme.split('-')[0] as ThemeFamily;
  invoke('set_tray_theme', { family }).catch(() => {
    // tray may not exist yet during early startup; ignore
  });
}

export function useTheme() {
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: queryKeys.settings.one(SETTING_KEY),
    queryFn: () => getSetting(SETTING_KEY),
  });

  const theme = (typeof data === 'string' ? (data as ThemeId) : null) ?? DEFAULT_THEME;

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  // Re-apply when the OS gets a focus event (other windows might have changed it).
  useEffect(() => {
    const refresh = () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(SETTING_KEY) });
    };
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, [queryClient]);

  const update = async (next: ThemeId) => {
    applyTheme(next);
    await setSetting(SETTING_KEY, next);
    queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(SETTING_KEY) });
  };

  return { theme, setTheme: update };
}
