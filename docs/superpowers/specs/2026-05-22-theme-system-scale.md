# Theme system scale-up — design

**Date:** 2026-05-22
**Scope:** Refactor the theming system so it scales cleanly to 8 theme families (16 variants), then add 4 new families: Wabi-Sabi, Risograph Zine, Constructivist, Mid-Century Atomic.

## Motivation

The current theme system has 4 families and is starting to strain:

- `app/src/index.css` is 569 lines, mixing tailwind directives, theme variable blocks, bespoke per-theme helper classes (`.holo-shimmer`, `.holo-card`, `.holo-frame`, `.memphis-dots`, `.memphis-grid`, `.memphis-card`, `.memphis-card-accent`), and theme-specific body effects (CRT scanlines on `robotic-dark`).
- `theme.ts` has a hand-maintained `ThemeFamily` union, a hand-maintained `ThemeId` union, and a hand-maintained `THEMES` array. Three places to edit per new family.
- `SettingsWindow.tsx` has its own `FAMILY_PREVIEW` map duplicating family-level metadata (palette swatches, fonts, shape) that should live with the family definition.

We want to double the family count. Doing it on the current structure would push `index.css` past 1000 lines and require edits in 4 separate places per theme. This refactor introduces a registry pattern so adding a theme is **two files: a CSS file and one registry entry**.

## Final structure

```
app/src/
  themes/
    _base.css          # .menu-divider base + shared keyframes (holo-flow)
    holo.css           # variables + .holo-shimmer + .holo-card + .holo-frame + divider
    memphis.css        # variables + .memphis-* + .holo-shimmer memphis variant + divider
    robotic.css        # variables + body scanlines + .holo-shimmer variant + divider
    corporate.css      # variables + .holo-shimmer variant + divider
    wabi-sabi.css      # NEW
    riso.css           # NEW
    constructivist.css # NEW
    atomic.css         # NEW
  index.css            # @tailwind + @import of themes + base layout
  lib/
    theme.ts           # THEME_FAMILIES registry, ThemeFamily/ThemeId derived types, THEMES derived array, family metadata
```

### Registry shape

```ts
// app/src/lib/theme.ts
export interface FamilyDef {
  label: string;          // user-visible name ("Holographic Dreamcore")
  tagline: string;        // one-line descriptor for the spec/preview
  preview: FamilyPreview; // palette swatches, fonts, shape — moved from SettingsWindow
}

export const THEME_FAMILIES = {
  holo:          { label: 'Holographic Dreamcore',  tagline: '...', preview: {...} },
  memphis:       { label: 'Memphis Machine',         tagline: '...', preview: {...} },
  robotic:       { label: 'Mr Robotic',              tagline: '...', preview: {...} },
  corporate:     { label: 'Corporate Overlord',      tagline: '...', preview: {...} },
  'wabi-sabi':   { label: 'Wabi-Sabi',               tagline: '...', preview: {...} },
  riso:          { label: 'Risograph Zine',          tagline: '...', preview: {...} },
  constructivist:{ label: 'Constructivist',          tagline: '...', preview: {...} },
  atomic:        { label: 'Mid-Century Atomic',      tagline: '...', preview: {...} },
} as const satisfies Record<string, FamilyDef>;

export type ThemeFamily = keyof typeof THEME_FAMILIES;
export type ThemeId = `${ThemeFamily}-light` | `${ThemeFamily}-dark`;

export const THEMES: ReadonlyArray<{ id: ThemeId; label: string; family: ThemeFamily; mode: 'light' | 'dark' }> =
  (Object.keys(THEME_FAMILIES) as ThemeFamily[]).flatMap((family) => [
    { id: `${family}-light` as ThemeId, label: `${THEME_FAMILIES[family].label} — Light`, family, mode: 'light' },
    { id: `${family}-dark`  as ThemeId, label: `${THEME_FAMILIES[family].label} — Dark`,  family, mode: 'dark' },
  ]);
```

`SettingsWindow.tsx` deletes its local `FAMILY_PREVIEW`, imports `THEME_FAMILIES` from `theme.ts`, and reads `THEME_FAMILIES[family].preview` in `ThemeCard`.

### Rust side

`app/src-tauri/src/main.rs` already takes `family: String` — no enum. The `tray_icon_for(&family)` function presumably has a match arm per family; needs new arms for the 4 new families. Worst case it falls back to a default icon for unknown families.

## Phase plan

**Phase 0 (this commit) — Infrastructure refactor, no UX or theme changes:**

1. Create `app/src/themes/{_base,holo,memphis,robotic,corporate}.css` by extracting blocks from `index.css`
2. `index.css` becomes tailwind directives + theme `@import`s + base layout only
3. Restructure `theme.ts` with registry pattern (derive ThemeId/THEMES from THEME_FAMILIES)
4. Move FAMILY_PREVIEW into `theme.ts` under `THEME_FAMILIES[family].preview`; SettingsWindow imports from there
5. Update `tray_icon_for` in Rust to handle unknown families gracefully (returns default icon) — already does this implicitly, just confirming
6. Verify: typecheck, all tests pass, smoke each existing theme in browser

**Phase 1–4 — One theme per phase, in order:**

For each new family (wabi-sabi, riso, constructivist, atomic):

1. Brainstorm palette + fonts + bespoke effects + divider in visual companion
2. Create `app/src/themes/<family>.css` with variables, body effects, divider override, any bespoke utility classes
3. Add `@import` line in `index.css`
4. Add entry to `THEME_FAMILIES` in `theme.ts` (label + tagline + preview)
5. (Rust) Add icon to `tray_icon_for` if we want family-specific tray icons; otherwise fall back to default
6. Verify visually
7. Commit

**Phase 5 — already done.** Settings card grid auto-picks up new themes once they're in `THEMES`. If grid feels too tall at 16 entries, we'd switch to 3 cols or collapse light/dark — defer until visual confirmation.

## Out of scope

- Renaming `.holo-shimmer` to `.theme-title` (good idea, but a separate refactor)
- Tray icon assets for the 4 new families (defer to the default icon for now)
- Theme preview animations / hover effects in the card grid

## Testing

Phase 0 is a pure refactor. Verification:
- `pnpm exec tsc --noEmit` clean
- `pnpm exec vitest run` all 131 tests pass (theme.test.tsx exercises the family slug derivation and `set_tray_theme` invocation)
- Manual: cycle through all 8 existing themes in Settings, confirm visual parity with pre-refactor
