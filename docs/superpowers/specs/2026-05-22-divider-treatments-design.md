# Sidebar divider treatments — design

**Date:** 2026-05-22
**Scope:** `.menu-divider` per-theme treatments for the library sidebar

## Motivation

The base `.menu-divider` is a thin hairline. Memphis already had a squiggle; holo got an iridescent gradient fade; robotic got a dashed terminal rule; corporate inherited the base hairline. The user judged the robotic dashed line and the corporate hairline "too boring" — both treatments solved the divider problem but did not carry the theme's voice. This change replaces both with treatments that read as *that theme's signature artifact* rather than a generic separator.

## Final treatments

| Theme family | Treatment | Visual |
|---|---|---|
| `memphis` | Hand-drawn squiggle (unchanged) | `~∼~∼~∼~∼~∼` |
| `holo` | Iridescent gradient fade (unchanged) | `░▒▓████▓▒░` |
| `robotic` | Hex memory dump | `0x00FF  41 6C 6C 0A 2D 54 41 47 53` |
| `corporate` | Stamped Roman serial, position-incrementing | `——— № I ———` / `——— № II ———` / `——— № III ———` |

Both light and dark variants of every theme adopt the same treatment; only colors/glow change per variant (via existing CSS variables).

### Robotic — hex memory dump

A row of monospace hex bytes that reads as `xxd`/`hexdump` output, in `--text-muted` color. The byte sequence `41 6C 6C 0A 2D 54 41 47 53` is ASCII `All\n-TAGS` — a deliberate easter egg that fits the screen-capture / tag-organizer domain.

- Font: `IBM Plex Mono` (already the body font for robotic), 9px, 1.5px letter-spacing
- No glow, no animation (motion was rejected as distracting)
- Same string for all three divider positions

### Corporate — stamped Roman serial

A thin `--border` hairline with a centered monospace text mark. The mark text is `№ ` + the divider's position as an uppercase Roman numeral. Position is derived from DOM order via CSS counters, so adding or removing dividers renumbers automatically.

- Mark font: `IBM Plex Mono`, 9px, 1.5px letter-spacing, 600 weight, uppercase, `--text-muted` color
- Mark sits above the hairline with `background: var(--bg-soft)` and horizontal padding, so the line appears to terminate cleanly on either side without needing explicit segment math
- Three divider positions in the library sidebar produce `№ I`, `№ II`, `№ III`

## Implementation

Single file: `app/src/index.css`. The `.menu-divider` class is already in place from the prior change; this revision replaces the `[data-theme^='robotic']` and `[data-theme^='corporate']` blocks.

### CSS counter setup

```css
aside { counter-reset: divider; }
.menu-divider { counter-increment: divider; }
```

The increment is universal — themes that don't surface the counter pay no cost. The reset on `aside` is sufficient because all three sidebar dividers are descendants of the same `<aside>` element in `Sidebar.tsx` (the third divider is nested in a `mt-auto` wrapper, but CSS counters traverse the whole subtree in DOM order).

### Robotic block

```css
html[data-theme^='robotic'] .menu-divider {
  display: flex;
  justify-content: center;
  align-items: center;
  font-family: 'IBM Plex Mono', ui-monospace, monospace;
  font-size: 9px;
  letter-spacing: 1.5px;
  color: var(--text-muted);
  user-select: none;
  background-image: none;
}
html[data-theme^='robotic'] .menu-divider::before {
  content: "0x00FF  41 6C 6C 0A 2D 54 41 47 53";
}
```

Override `background-image: none` because the base sets a hairline that we no longer want.

### Corporate block

```css
html[data-theme^='corporate'] .menu-divider {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  background-image: linear-gradient(to right, var(--border), var(--border));
  background-size: 100% 1px;
  background-position: center;
  background-repeat: no-repeat;
}
html[data-theme^='corporate'] .menu-divider::before {
  content: "№ " counter(divider, upper-roman);
  background: var(--bg-soft);
  padding: 0 8px;
  font-family: 'IBM Plex Mono', ui-monospace, monospace;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 1.5px;
  color: var(--text-muted);
  text-transform: uppercase;
}
```

The mark's `background: var(--bg-soft)` matches the sidebar's `bg-bg-soft` Tailwind background, masking the hairline behind the text so the line appears to terminate on either side.

### What stays the same

- `.menu-divider` base class with default hairline (still serves as fallback for unknown themes)
- Memphis squiggle blocks (`html[data-theme^='memphis'] .menu-divider`, `html[data-theme='memphis-dark'] .menu-divider`)
- Holo gradient fade blocks (`html[data-theme^='holo'] .menu-divider`, `html[data-theme='holo-dark'] .menu-divider`)
- `Sidebar.tsx` — no React changes needed

## Out of scope

- Per-position byte variation for the robotic hex dump (could spell "All\nTAGS" / "TAGS\nCLIP" / "CLIP\nSETS" using CSS `:nth-of-type` tricks, but adds fragility for a small gain)
- Animations on either treatment (motion explicitly rejected)
- Use of the serial mark or hex dump outside the sidebar (the class is sidebar-scoped via the `aside` counter-reset)

## Testing

This is a CSS-only change. Verification is visual across 8 theme variants:

1. Cycle through all 8 themes in Settings, eyeballing the three dividers in the library sidebar
2. Confirm corporate `№ I` / `№ II` / `№ III` numbering matches DOM order
3. Confirm robotic hex string is readable on both robotic-light (amber on bone) and robotic-dark (amber on jet)
4. Confirm corporate mark background masks the hairline cleanly on both light and dark variants

No automated tests — the existing Sidebar tests don't assert on divider rendering, and CSS counters / pseudo-element content aren't testable in happy-dom anyway.
