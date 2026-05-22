# Divider Treatments Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Replace the boring robotic dashed-rule and corporate hairline `.menu-divider` treatments with theme-signature artifacts: a hex memory dump for robotic and a stamped Roman serial mark for corporate.

**Architecture:** Single CSS file change in `app/src/index.css`. Adds a `counter-reset: divider` on `<aside>` and `counter-increment` on `.menu-divider` (universal — costs nothing for themes that don't surface it). Replaces the existing `[data-theme^='robotic']` and `[data-theme^='corporate']` divider blocks with new flex+pseudo-element rules that emit text content via `::before`. No React changes; the existing `<div className="menu-divider" />` markup in `Sidebar.tsx` is sufficient.

**Tech Stack:** CSS (no framework). Uses Tailwind only for the surrounding sidebar layout, which is untouched.

**Spec:** [`docs/superpowers/specs/2026-05-22-divider-treatments-design.md`](../specs/2026-05-22-divider-treatments-design.md)

---

## Task 1: Update CSS divider blocks

**Files:**
- Modify: `app/src/index.css` (the `.menu-divider` section, currently at lines ~343–393)

**Step 1: Add counter scaffolding**

In `app/src/index.css`, locate the `.menu-divider` base block:

```css
/* Menu divider — theme-aware. Default is a thin hairline (used by corporate);
   each non-corporate theme overrides to match its visual language. */
.menu-divider {
  height: 12px;
  background-image: linear-gradient(to right, var(--border), var(--border));
  background-repeat: no-repeat;
  background-size: 100% 1px;
  background-position: center;
}
```

Add `counter-increment: divider;` inside `.menu-divider`, and add a new rule above it that resets the counter on `aside`. Final block:

```css
/* Menu divider — theme-aware. Default is a thin hairline (used as a safe
   fallback for any theme that doesn't override). Each theme below overrides
   to match its visual language. The CSS counter `divider` is used by themes
   that surface position (corporate's Roman serial). */
aside { counter-reset: divider; }

.menu-divider {
  height: 12px;
  counter-increment: divider;
  background-image: linear-gradient(to right, var(--border), var(--border));
  background-repeat: no-repeat;
  background-size: 100% 1px;
  background-position: center;
}
```

**Step 2: Replace the robotic block**

Find the existing robotic block in `app/src/index.css`:

```css
/* Robotic — dashed terminal rule; dark variant gets a soft amber phosphor glow */
html[data-theme^='robotic'] .menu-divider {
  background-image: repeating-linear-gradient(
    to right,
    var(--border) 0,
    var(--border) 8px,
    transparent 8px,
    transparent 14px
  );
  background-size: 100% 2px;
}
html[data-theme='robotic-dark'] .menu-divider {
  filter: drop-shadow(0 0 2px rgba(255, 176, 0, 0.5));
}
```

Replace it with:

```css
/* Robotic — hex memory dump (xxd / hexdump output styling).
   Bytes 41 6C 6C 0A 2D 54 41 47 53 = "All\n-TAGS" (deliberate easter egg). */
html[data-theme^='robotic'] .menu-divider {
  display: flex;
  justify-content: center;
  align-items: center;
  font-family: 'IBM Plex Mono', ui-monospace, monospace;
  font-size: 9px;
  letter-spacing: 1.5px;
  color: var(--text-muted);
  background-image: none;
  user-select: none;
}
html[data-theme^='robotic'] .menu-divider::before {
  content: "0x00FF  41 6C 6C 0A 2D 54 41 47 53";
}
```

Note: the existing `robotic-dark` glow rule is dropped — the user explicitly rejected motion/glow on the hex dump because the bytes themselves are the visual interest.

**Step 3: Replace the corporate block**

The corporate theme currently inherits the base hairline (no explicit override). Add a new block after the holo blocks:

```css
/* Corporate — stamped Roman serial. Counter increments per divider position
   (I, II, III) over a centered hairline. The mark uses bg-soft background to
   mask the hairline so the rule appears to terminate cleanly on each side. */
html[data-theme^='corporate'] .menu-divider {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
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
  z-index: 1;
}
```

**Step 4: Verify the file compiles / app type-checks**

Run from `app/`:

```bash
pnpm exec tsc --noEmit
```

Expected: clean exit (no output).

Run the test suite to confirm nothing broke:

```bash
pnpm exec vitest run
```

Expected: all tests pass (127 prior to this change).

**Step 5: Commit**

```bash
git add app/src/index.css docs/superpowers/specs/2026-05-22-divider-treatments-design.md docs/superpowers/plans/2026-05-22-divider-treatments.md
git commit -m "$(cat <<'EOF'
feat(ui): per-theme divider treatments for robotic + corporate

Robotic dividers now render as a hex memory dump in IBM Plex Mono;
corporate dividers render as a stamped Roman serial that auto-increments
per position via CSS counter. Memphis squiggle and holo gradient fade
are unchanged. Replaces the previous dashed-rule (robotic) and inherited
hairline (corporate) which carried no theme voice.
EOF
)"
```

---

## Task 2: Manual visual verification

CSS pseudo-element content and counters aren't testable in happy-dom. Verification is human-eyes-on across all 8 theme variants.

**Files:**
- Run: dev server via `pnpm tauri dev` from repo root, OR open the existing dev session if one is running

**Step 1: Launch the app (if not already running)**

From repo root:

```bash
pnpm tauri dev
```

Wait for the library window to appear.

**Step 2: Walk through each theme**

Open Settings (gear icon at the bottom of the sidebar) and cycle through all 8 themes:

1. Holographic Dreamcore — Light
2. Holographic Dreamcore — Dark
3. Memphis Machine — Light
4. Memphis Machine — Dark
5. Mr Robotic — Light
6. Mr Robotic — Dark
7. Corporate Overlord — Light
8. Corporate Overlord — Dark

For each, look at the library sidebar and check:

- **Holo + Memphis:** dividers are unchanged from prior commit (iridescent gradient / hand-drawn squiggle)
- **Robotic (both):** all three dividers show `0x00FF  41 6C 6C 0A 2D 54 41 47 53` in `--text-muted` color, centered, monospace. No glow.
- **Corporate (both):** three dividers show `№ I`, `№ II`, `№ III` respectively (in DOM order), centered on a hairline that visually terminates on either side of the mark.

**Step 3: Confirm masking works**

On corporate-light and corporate-dark, zoom in on the corporate divider. The hairline should appear to stop ~8px before the `№ X` text and resume ~8px after. If the line cuts through the text instead, the mark's `background: var(--bg-soft)` color isn't matching the sidebar background — investigate.

**Step 4: Smoke the live theme switch**

With multiple windows open (Library + Settings), change the theme. Both windows should restyle immediately (the cross-window theme broadcast was added earlier in this session). If the gallery sidebar doesn't update, that's a separate regression, not a divider issue.

---

## Self-review

- **Spec coverage:** Hex dump (robotic): Step 2. Roman serial (corporate): Step 3. CSS counter approach: Step 1. Masking via bg-soft: Step 3 + Step 3 of Task 2. ✓
- **Placeholder scan:** No TBDs, no "add appropriate handling" — every step has exact code. ✓
- **Buildability:** A fresh engineer can copy each code block directly. The hex string and the counter syntax are both load-bearing — both are spelled out. ✓
- **Task decomposition:** Two tasks, clean boundary (CSS change vs visual verification). ✓
