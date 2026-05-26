# React Theme Picker Wizard Step (PR C) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:team-driven-development to implement this plan.

**Goal:** Add a theme picker step to FirstRunWizard so first-launch users choose their theme before reaching the main app, closing #87.

**Architecture:** Inline new `'theme'` step in `FirstRunWizard.tsx` between `'welcome'` and `'hotkeys'`. Reuse the existing `useTheme()` hook from `app/src/lib/theme.ts` — `setTheme(id)` already calls `applyTheme(id)` synchronously (live preview) and persists via `setSetting('theme', id)` async. Render an 8-card grid (one per theme family, all defaulting to the `-dark` variant; user can switch to light or different family from Settings later). Each card is a button styled with the family's accent color; the currently-selected card gets a ring.

**Tech Stack:** React 18, TypeScript strict, existing `useTheme()` hook, existing `THEME_FAMILIES`/`THEMES` registry.

**Spec:** #87 + cluster design at `docs/superpowers/specs/2026-05-25-react-cluster-design.md` ("Theme picker: step 2 of FirstRunWizard, after Welcome. Radio-style cards with live preview on selection. Persists via existing theme settings command on Next.")

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker/`
**Branch:** `feat/react-theme-picker` (off `origin/main` after PR A + PR B merged)
**Baseline:** 225/225 app tests + 38 package tests = 263 TS passing.

---

## Conventions

- Conventional Commits: `feat(ui):`, `test(ui):`.
- Stage explicit paths, NEVER `git add .` or `-A`.
- One task = one commit.
- No comments unless WHY is non-obvious.
- TS strict + noUncheckedIndexedAccess.

## Dependency graph

```
T1 (Wizard step + tests) → T2 (final verification)
```

Simple linear plan. Single implementer can handle both tasks.

---

## Task 1: ThemePickerStep inline in FirstRunWizard

**Files:**
- Modify: `app/src/windows/library/FirstRunWizard.tsx` — add `'theme'` to step union, add step UI, wire `useTheme()`
- Modify: `app/src/windows/library/FirstRunWizard.test.tsx` — migrate existing 4 tests to `renderWithQuery` (FirstRunWizard now uses useQuery via useTheme), update flow assertions for the new step, add 2 new tests for the theme step

**Context:** `useTheme()` (in `app/src/lib/theme.ts:261`) is a React hook that calls `useQuery`. Calling it inside FirstRunWizard means the wizard needs a QueryClientProvider in its tests — `renderWithQuery` from `app/src/test/renderWithQuery.tsx` provides one. The existing 4 tests use plain `render` and would throw `No QueryClient set` once the wizard pulls in `useTheme`.

The flow after this change: `welcome → theme → hotkeys → library → done`.

**Step 1: Write the failing tests**

Replace `app/src/windows/library/FirstRunWizard.test.tsx` with:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { FirstRunWizard } from './FirstRunWizard';
import { renderWithQuery } from '../../test/renderWithQuery';

describe('<FirstRunWizard />', () => {
  it('starts on the welcome step', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
    expect(screen.getByText(/get started/i)).toBeInTheDocument();
  });

  it('walks through welcome → theme → hotkeys → library → done', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    expect(screen.getByRole('heading', { name: /pick a theme/i })).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    expect(screen.getByText('Capture region')).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/library location/i)).toBeInTheDocument();
    expect(screen.getByText(/%APPDATA%/i)).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/all set/i)).toBeInTheDocument();
  });

  it('back buttons return to the previous step', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i)); // -> hotkeys
    fireEvent.click(screen.getByText(/^next$/i)); // -> library
    fireEvent.click(screen.getByText(/back/i));   // -> hotkeys
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    fireEvent.click(screen.getByText(/back/i));   // -> theme
    expect(screen.getByRole('heading', { name: /pick a theme/i })).toBeInTheDocument();
    fireEvent.click(screen.getByText(/back/i));   // -> welcome
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
  });

  it("calls setSetting('firstrun.completed', true) and onComplete on the final button", async () => {
    const onComplete = vi.fn();
    renderWithQuery(<FirstRunWizard onComplete={onComplete} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i)); // theme -> hotkeys
    fireEvent.click(screen.getByText(/^next$/i)); // hotkeys -> library
    fireEvent.click(screen.getByText(/^next$/i)); // library -> done
    fireEvent.click(screen.getByText(/start using snapper-keeper/i));

    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'firstrun.completed',
        value: true,
      });
      expect(onComplete).toHaveBeenCalled();
    });
  });

  it('theme step renders one card per family', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    // 8 families = 8 cards. Each card has a data-testid="theme-card".
    const cards = screen.getAllByTestId('theme-card');
    expect(cards).toHaveLength(8);
  });

  it('clicking a theme card persists the selection via setSetting', async () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    // Click the second card (any non-default).
    const cards = screen.getAllByTestId('theme-card');
    fireEvent.click(cards[1]!);
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        'plugin:snk-library|set_setting',
        expect.objectContaining({ key: 'theme' }),
      );
    });
  });
});
```

**Step 2: Run tests, verify they fail**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm --filter @snk/app test -- --run src/windows/library/FirstRunWizard.test.tsx
```

Expected: 6 failing tests (2 new tests reference `theme-card` testids that don't exist; flow tests expect the theme step heading that doesn't render yet).

**Step 3: Add the theme step to FirstRunWizard.tsx**

Modify `app/src/windows/library/FirstRunWizard.tsx`:

1. Add imports at the top, after the existing imports:

```tsx
import { THEME_FAMILIES, useTheme, type ThemeFamily, type ThemeId } from '../../lib/theme';
```

2. Change the `Step` type union to include `'theme'`:

```tsx
type Step = 'welcome' | 'theme' | 'hotkeys' | 'library' | 'done';
```

3. Inside `FirstRunWizard`, after the existing `const [step, setStep] = useState<Step>('welcome');`, add:

```tsx
  const { theme, setTheme } = useTheme();
  const currentFamily = theme.replace(/-(light|dark)$/, '') as ThemeFamily;
```

4. Update the welcome step's `get started` button click handler from `setStep('hotkeys')` to `setStep('theme')`.

5. Add the new `'theme'` step JSX, between the welcome step block (which ends with the closing `)}`) and the hotkeys step block. Insert this immediately after the welcome block:

```tsx
        {step === 'theme' && (
          <div className="space-y-5">
            <h2 className="font-display text-xl">🎨 pick a theme</h2>
            <p className="text-sm text-fg-muted">
              Choose one — you can change it anytime in Settings.
            </p>
            <div className="grid grid-cols-2 gap-3">
              {(Object.keys(THEME_FAMILIES) as ThemeFamily[]).map((family) => {
                const def = THEME_FAMILIES[family];
                const id = `${family}-dark` as ThemeId;
                const active = currentFamily === family;
                return (
                  <button
                    key={family}
                    data-testid="theme-card"
                    onClick={() => setTheme(id)}
                    className={`text-left p-3 rounded-xl border-2 transition-transform hover:-translate-y-0.5 ${
                      active ? 'border-primary ring-2 ring-primary' : 'border-border'
                    }`}
                    style={{ background: def.preview.bgDark }}
                  >
                    <div
                      className="text-sm"
                      style={{ fontFamily: def.preview.displayFont, color: def.preview.fgDark }}
                    >
                      {def.label}
                    </div>
                    <div
                      className="text-[10px] mt-1 italic"
                      style={{ fontFamily: def.preview.bodyFont, color: def.preview.mutedDark }}
                    >
                      {def.tagline}
                    </div>
                  </button>
                );
              })}
            </div>
            <div className="flex justify-between items-center">
              <button
                className="text-sm text-fg-muted hover:text-fg"
                onClick={() => setStep('welcome')}
              >
                ← back
              </button>
              <PrimaryButton onClick={() => setStep('hotkeys')}>next</PrimaryButton>
            </div>
          </div>
        )}
```

6. Update the hotkeys step's back button handler from `setStep('welcome')` to `setStep('theme')`.

**Step 4: Run tests, verify pass**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm --filter @snk/app test -- --run src/windows/library/FirstRunWizard.test.tsx
```

Expected: 6/6 passing.

**Step 5: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 6: Commit**

```bash
git add app/src/windows/library/FirstRunWizard.tsx app/src/windows/library/FirstRunWizard.test.tsx
git commit -m "feat(ui): add theme picker step to FirstRunWizard (closes #87)"
```

---

## Task 2: Final verification

**Files:** None modified. Verification only.

**Step 1: Full TS suite**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm -r --filter "@snk/*" --filter @snk/app test
```

Expected: ~265 tests passing (263 baseline + 2 new FirstRunWizard tests, plus the migrated 4 still count as 4). Total 265.

**Step 2: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 3: Vite build**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-theme-picker
pnpm --filter @snk/app build
```

Expected: succeeds.

**Step 4: Hand off**

Report all-green to spec-auditor + quality-sentinel.

---

## Self-review notes

1. **Spec coverage:** Theme step inserted between welcome and hotkeys (#87 + design). Live preview via `setTheme()`'s synchronous `applyTheme()` (existing behavior). Persistence via `setSetting('theme', id)` (existing behavior, unchanged).
2. **Placeholders:** none.
3. **Naming consistency:** `Step` union has `'theme'` as the new variant; matches the JSX `step === 'theme'` check. `data-testid="theme-card"` matches the test selectors.
4. **Buildability:** Single implementer task; the theme step is ~35 lines added; existing tests migrate to renderWithQuery + 2 new tests added.

## Plan-as-source-of-truth reminder

Real bugs → SendMessage `team-lead` BEFORE applying. Per memory `[[feedback_plan_as_source_of_truth]]`.
