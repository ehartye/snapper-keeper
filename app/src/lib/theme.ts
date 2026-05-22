import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from './queryKeys';

export const THEME_CHANGED_EVENT = 'theme:changed';

// ----------------------------------------------------------------------------
// Theme family registry
// ----------------------------------------------------------------------------
// Adding a new theme is three steps:
//   1. Create app/src/themes/<family>.css with the variable blocks +
//      bespoke utility classes + .menu-divider override.
//   2. Add an @import line in app/src/index.css.
//   3. Add an entry below — label, tagline, and the preview metadata used by
//      the Settings ThemeCard grid.
// `ThemeFamily`, `ThemeId`, and `THEMES` are all derived from this registry.

export type CardShape = 'round' | 'memphis' | 'terminal' | 'card';
export type SwatchShape = 'round' | 'square';

export interface FamilyPreview {
  swatches: string[];
  bgLight: string;
  bgDark: string;
  fgLight: string;
  fgDark: string;
  mutedLight: string;
  mutedDark: string;
  displayFont: string;
  bodyFont: string;
  shape: CardShape;
  swatchShape: SwatchShape;
}

export interface FamilyDef {
  label: string;
  tagline: string;
  preview: FamilyPreview;
}

export const THEME_FAMILIES = {
  holo: {
    label: 'Holographic Dreamcore',
    tagline: 'Lisa Frank holographic foil, cotton-candy gradients, peak \'90s sticker book',
    preview: {
      swatches: ['#ff2d95', '#c77dff', '#00e5ff', '#7affd7'],
      bgLight: 'linear-gradient(135deg, #fff5f7, #ffe4ec)',
      bgDark: 'linear-gradient(135deg, #1a0b2e, #3d1a66)',
      fgLight: '#3d1a4d',
      fgDark: '#ffe5f1',
      mutedLight: '#8b6da6',
      mutedDark: '#c9a3d6',
      displayFont: "'Rubik Bubbles', sans-serif",
      bodyFont: "'Fredoka', sans-serif",
      shape: 'round',
      swatchShape: 'round',
    },
  },
  memphis: {
    label: 'Memphis Machine',
    tagline: 'Memphis Group geometric chaos, hot primaries, squiggles, hard angles',
    preview: {
      swatches: ['#ff3838', '#ffd93d', '#2e5bff', '#00b894'],
      bgLight: '#ffffff',
      bgDark: '#1f1f1f',
      fgLight: '#0a0a0a',
      fgDark: '#fafaf5',
      mutedLight: '#555550',
      mutedDark: '#9a9a92',
      displayFont: "'Bungee', sans-serif",
      bodyFont: "'IBM Plex Mono', monospace",
      shape: 'memphis',
      swatchShape: 'square',
    },
  },
  robotic: {
    label: 'Mr Robotic',
    tagline: 'CRT amber phosphor, scanlines, terminal glow, fsociety',
    preview: {
      swatches: ['#ffb000', '#00ff41', '#00d4ff', '#ff003c'],
      bgLight: '#f5efde',
      bgDark: '#0a0a0a',
      fgLight: '#3a2800',
      fgDark: '#ffb000',
      mutedLight: '#7a5d24',
      mutedDark: '#8a6500',
      displayFont: "'VT323', monospace",
      bodyFont: "'IBM Plex Mono', monospace",
      shape: 'terminal',
      swatchShape: 'square',
    },
  },
  corporate: {
    label: 'Corporate Overlord',
    tagline: 'Industrial-luxury brutalism, jet black on bone white, severe red',
    preview: {
      swatches: ['#09090b', '#1e3a5f', '#991b1b', '#71717a'],
      bgLight: '#ffffff',
      bgDark: '#18181b',
      fgLight: '#09090b',
      fgDark: '#fafafa',
      mutedLight: '#52525b',
      mutedDark: '#a1a1aa',
      displayFont: "'Big Shoulders Display', sans-serif",
      bodyFont: "'Archivo', sans-serif",
      shape: 'card',
      swatchShape: 'square',
    },
  },
  'wabi-sabi': {
    label: 'Wabi-Sabi',
    tagline: 'Washi paper and sumi ink. Hanko stamps and wandering brush strokes.',
    preview: {
      swatches: ['#c54a3a', '#8a6f4c', '#3a4a5e', '#5a6e3e'],
      bgLight: '#f5efe0',
      bgDark: '#14100c',
      fgLight: '#1a1612',
      fgDark: '#e8d8c0',
      mutedLight: '#6e5a48',
      mutedDark: '#8a7560',
      displayFont: "'Shippori Mincho', 'Yu Mincho', serif",
      bodyFont: "'Cormorant Garamond', Georgia, serif",
      shape: 'card',
      swatchShape: 'round',
    },
  },
  riso: {
    label: 'Risograph Zine',
    tagline: 'Duotone print plus intentional misregistration. Fluorescent inks, halftone dots.',
    preview: {
      swatches: ['#ff5589', '#1f4587', '#3d7e47', '#d4a024'],
      bgLight: '#f5f0e5',
      bgDark: '#0d0d0d',
      fgLight: '#1a1f2a',
      fgDark: '#fbf7ed',
      mutedLight: '#5a6478',
      mutedDark: '#8a7a6c',
      displayFont: "'Bebas Neue', sans-serif",
      bodyFont: "'Karla', sans-serif",
      shape: 'card',
      swatchShape: 'square',
    },
  },
} as const satisfies Record<string, FamilyDef>;

export type ThemeFamily = keyof typeof THEME_FAMILIES;
export type ThemeId = `${ThemeFamily}-light` | `${ThemeFamily}-dark`;
export type ThemeMode = 'light' | 'dark';

export interface ThemeEntry {
  id: ThemeId;
  label: string;
  family: ThemeFamily;
  mode: ThemeMode;
}

export const THEMES: ReadonlyArray<ThemeEntry> = (Object.keys(THEME_FAMILIES) as ThemeFamily[])
  .flatMap((family): ThemeEntry[] => {
    const def = THEME_FAMILIES[family];
    return [
      { id: `${family}-light` as ThemeId, label: `${def.label} — Light`, family, mode: 'light' },
      { id: `${family}-dark` as ThemeId, label: `${def.label} — Dark`, family, mode: 'dark' },
    ];
  });

export const DEFAULT_THEME: ThemeId = 'holo-dark';

/** Returns the family slug for a theme id. Handles multi-hyphen family
 *  slugs (e.g. wabi-sabi-light → wabi-sabi). */
export function familyOf(theme: ThemeId): ThemeFamily {
  return theme.replace(/-(light|dark)$/, '') as ThemeFamily;
}

// ----------------------------------------------------------------------------
// Theme application + persistence
// ----------------------------------------------------------------------------

const SETTING_KEY = 'theme';

function applyTheme(theme: ThemeId) {
  document.documentElement.setAttribute('data-theme', theme);
  // Tray + window icons are global; only the library window broadcasts the
  // change to Rust so we don't spam the IPC with one call per window.
  if (getCurrentWindow().label !== 'library') return;
  const family = familyOf(theme);
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

  // Re-apply on focus (backup) and whenever any window broadcasts a theme change.
  useEffect(() => {
    const refresh = () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(SETTING_KEY) });
    };
    window.addEventListener('focus', refresh);

    let unlisten: (() => void) | undefined;
    listen(THEME_CHANGED_EVENT, refresh)
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    return () => {
      window.removeEventListener('focus', refresh);
      unlisten?.();
    };
  }, [queryClient]);

  const update = async (next: ThemeId) => {
    applyTheme(next);
    await setSetting(SETTING_KEY, next);
    await emit(THEME_CHANGED_EVENT, next);
    queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(SETTING_KEY) });
  };

  return { theme, setTheme: update };
}
