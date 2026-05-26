# ClipboardSettings Polish (PR E) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:team-driven-development to implement this plan.

**Goal:** Polish `ClipboardSettings.tsx` for design-system consistency and modal keyboard handling. Closes #95.

**Two related items:**
1. **Header styling normalization** — current `<h2>` uses `text-sm font-display uppercase tracking-wider text-fg-muted mb-2`; match the rest of Settings (`font-display text-sm mb-3` per `SettingsSection`).
2. **Modal keyboard shortcuts via shared `Modal`** — migrate inline `AddAppModal` and `ConfirmFrontmostModal` to `useModal().custom({...})` from PR A. Free Esc-to-close, focus trap, focus return.

**Architecture:** Use `modal.custom()` for both modals (vs `.confirm`) because `AddAppModal` has a form and `ConfirmFrontmostModal` has a dup-check disabled state — both need more control than `.confirm` exposes. The current inner `AddAppModal` and `ConfirmFrontmostModal` functions are renamed to `AddAppForm` and `ConfirmFrontmostBody`, the outer `<div className="fixed inset-0 bg-black/40 …">` backdrop is removed (Modal provides that), and the open/close state moves from `useState` to the modal API.

**Tech Stack:** React 18, TypeScript strict, shared `Modal` + `useModal` from PR A.

**Spec:** #95 (design-system unification + modal keyboard shortcuts; preserve 6 existing vitest cases).

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish/`
**Branch:** `feat/react-clipboard-polish` (off origin/main after PRs A/B/C merged)
**Baseline:** 265 TS tests passing (227 app + 38 packages).

---

## Conventions

- `refactor(settings):` scope (no semantic change, only structure + styling).
- Stage explicit paths, NEVER `git add .` or `-A`.
- One task = one commit.
- No comments unless WHY is non-obvious.

## Dependency graph

```
T1 (ClipboardSettings refactor) → T2 (verify)
```

Single implementer. Linear.

---

## Task 1: ClipboardSettings refactor

**Files:**
- Modify: `app/src/windows/settings/ClipboardSettings.tsx`
- Modify: `app/src/windows/settings/ClipboardSettings.test.tsx`

**Context:**
- PR A's `<ModalProvider>` is already wired at `app/src/main.tsx` (PR B verified this) — runtime is fine.
- ClipboardSettings is rendered inside SettingsWindow, which is rendered inside `<ModalProvider>` (PR B already added the wrap to SettingsWindow.test.tsx for the same reason).
- ClipboardSettings tests render `<ClipboardSettings />` directly via `renderWithQuery`. Once `useModal()` is called inside, these tests need a `<ModalProvider>` wrapper too.
- `ClipboardSettings.readEntries.test.ts` is a pure-fn test on `readEntries` — no component render — needs no change.

**Step 1: Update existing tests to wrap with ModalProvider + add modal-root setup**

Modify `app/src/windows/settings/ClipboardSettings.test.tsx`. At top, add the import:

```tsx
import { ModalProvider } from '../../components/Modal';
```

Find the existing `beforeEach` (or add one if absent) and ensure it includes a `modal-root` div:

```tsx
beforeEach(() => {
  mockedInvoke.mockReset();
  mockedInvoke.mockResolvedValue(null);

  const existing = document.getElementById('modal-root');
  if (existing) existing.remove();
  const root = document.createElement('div');
  root.id = 'modal-root';
  document.body.appendChild(root);
});
```

(Keep any other reset logic already present.)

Replace each `renderWithQuery(<ClipboardSettings />)` (all occurrences) with:

```tsx
renderWithQuery(<ModalProvider><ClipboardSettings /></ModalProvider>);
```

The 6 existing test bodies stay otherwise unchanged — they assert on the rendered list and the persistence calls, both of which remain after the refactor.

**Step 2: Run existing tests, verify they still pass (regression baseline)**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm --filter @snk/app test -- --run src/windows/settings/ClipboardSettings.test.tsx src/windows/settings/ClipboardSettings.readEntries.test.ts
```

Expected: 6 + 10 = 16 tests passing. (They pass with just the wrap added because the refactor in Step 3 hasn't happened yet; the inline modals still exist and the open/close state still works the old way.)

**Step 3: Refactor `ClipboardSettings.tsx`**

Replace the entire contents of `app/src/windows/settings/ClipboardSettings.tsx` with:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import { getSetting, setSetting } from '@snk/library';
import {
  APP_BLOCKLIST_SETTING_KEY,
  detectFrontmostApp,
  type BlocklistEntry,
  type SourceApp,
} from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';
import { useModal } from '../../components/Modal';
import { Button } from '../../components/Button';

export function readEntries(value: unknown): BlocklistEntry[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (e: unknown): e is BlocklistEntry =>
      typeof e === 'object' &&
      e !== null &&
      typeof (e as { identifier?: unknown }).identifier === 'string' &&
      typeof (e as { display_name?: unknown }).display_name === 'string' &&
      ((e as { kind?: unknown }).kind === 'macos_bundle_id' ||
        (e as { kind?: unknown }).kind === 'windows_exe'),
  );
}

export function ClipboardSettings() {
  const queryClient = useQueryClient();
  const modal = useModal();
  const { data: rawValue } = useQuery({
    queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    queryFn: () => getSetting(APP_BLOCKLIST_SETTING_KEY),
  });
  const entries = readEntries(rawValue);

  async function persist(next: BlocklistEntry[]) {
    await setSetting(APP_BLOCKLIST_SETTING_KEY, next);
    await queryClient.invalidateQueries({
      queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    });
  }

  function remove(identifier: string, kind: BlocklistEntry['kind']) {
    void persist(
      entries.filter((e) => !(e.identifier === identifier && e.kind === kind)),
    );
  }

  function openAddApp() {
    modal.custom({
      title: 'Add excluded app',
      render: ({ close }) => (
        <AddAppForm
          existing={entries}
          onAdd={(entry) => {
            void persist([...entries, entry]);
            close();
          }}
          onCancel={close}
        />
      ),
    });
  }

  async function addFromFrontmost() {
    const app = await detectFrontmostApp();
    if (!app) return;
    modal.custom({
      title: 'Block frontmost app?',
      render: ({ close }) => (
        <ConfirmFrontmostBody
          app={app}
          existing={entries}
          onConfirm={(entry) => {
            void persist([...entries, entry]);
            close();
          }}
          onCancel={close}
        />
      ),
    });
  }

  return (
    <div>
      <h2 className="font-display text-sm mb-3">Excluded apps</h2>
      <p className="text-[11px] text-fg-muted mb-3">
        Clipboard events from these apps are never recorded. OS-level
        &quot;concealed&quot; flags are always honored regardless of this list.
      </p>

      <ul className="border border-border rounded">
        {entries.length === 0 && (
          <li className="px-3 py-2 text-xs text-fg-muted">
            No exclusions configured.
          </li>
        )}
        {entries.map((e) => (
          <li
            key={`${e.kind}:${e.identifier}`}
            className="flex items-center justify-between px-3 py-2 border-b border-border last:border-0"
          >
            <div>
              <div className="text-sm text-fg">{e.display_name}</div>
              <div className="text-[10px] text-fg-muted">
                {e.identifier} · {e.kind}
              </div>
            </div>
            <button
              onClick={() => remove(e.identifier, e.kind)}
              className="text-fg-muted hover:text-danger text-xs"
              aria-label={`Remove ${e.display_name}`}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      <div className="flex gap-2 mt-3">
        <button
          onClick={openAddApp}
          className="text-xs text-fg hover:text-primary"
        >
          + Add app…
        </button>
        <button
          onClick={addFromFrontmost}
          className="text-xs text-fg hover:text-primary"
        >
          + Add from frontmost app
        </button>
      </div>
    </div>
  );
}

interface AddAppFormProps {
  existing: BlocklistEntry[];
  onAdd: (entry: BlocklistEntry) => void;
  onCancel: () => void;
}

function AddAppForm({ existing, onAdd, onCancel }: AddAppFormProps) {
  const [identifier, setIdentifier] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [kind, setKind] = useState<BlocklistEntry['kind']>('macos_bundle_id');
  const [error, setError] = useState<string | null>(null);

  function submit() {
    const id = identifier.trim();
    if (!id) {
      setError('Identifier is required.');
      return;
    }
    const dup = existing.find((e) => e.identifier === id && e.kind === kind);
    if (dup) {
      setError('Already in the list.');
      return;
    }
    onAdd({
      identifier: id,
      display_name: displayName.trim() || id,
      kind,
    });
  }

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <label className="block text-[10px] text-fg-muted mb-1">Kind</label>
      <select
        value={kind}
        onChange={(e) => setKind(e.target.value as BlocklistEntry['kind'])}
        className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
      >
        <option value="macos_bundle_id">macOS bundle ID</option>
        <option value="windows_exe">Windows exe</option>
      </select>
      <label className="block text-[10px] text-fg-muted mb-1">Identifier</label>
      <input
        value={identifier}
        onChange={(e) => setIdentifier(e.target.value)}
        placeholder={kind === 'macos_bundle_id' ? 'com.example.app' : 'example.exe'}
        className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
      />
      <label className="block text-[10px] text-fg-muted mb-1">
        Display name (optional)
      </label>
      <input
        value={displayName}
        onChange={(e) => setDisplayName(e.target.value)}
        className="w-full text-xs bg-surface border border-border rounded p-1 mb-3"
      />
      {error && <div className="text-[10px] text-danger mb-2">{error}</div>}
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">Add</Button>
      </div>
    </form>
  );
}

interface ConfirmFrontmostBodyProps {
  app: SourceApp;
  existing: BlocklistEntry[];
  onConfirm: (entry: BlocklistEntry) => void;
  onCancel: () => void;
}

function ConfirmFrontmostBody({
  app,
  existing,
  onConfirm,
  onCancel,
}: ConfirmFrontmostBodyProps) {
  const dup = existing.find(
    (e) => e.identifier === app.identifier && e.kind === app.kind,
  );
  return (
    <div>
      <div className="text-sm text-fg mb-1">{app.display_name}</div>
      <div className="text-[10px] text-fg-muted mb-3">
        {app.identifier} · {app.kind}
      </div>
      {dup && (
        <div className="text-[10px] text-danger mb-2">
          This app is already in the list.
        </div>
      )}
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          disabled={!!dup}
          onClick={() =>
            onConfirm({
              identifier: app.identifier,
              display_name: app.display_name,
              kind: app.kind,
            })
          }
        >
          Add
        </Button>
      </div>
    </div>
  );
}
```

**Step 4: Re-run the tests (now using shared Modal)**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm --filter @snk/app test -- --run src/windows/settings/ClipboardSettings.test.tsx src/windows/settings/ClipboardSettings.readEntries.test.ts
```

Expected: 6 + 10 = 16 tests passing. The 6 component tests already exercise add-app and confirm-frontmost flows; they continue to work because:
- Both buttons still exist with the same labels (`+ Add app…`, `+ Add from frontmost app`)
- The form fields inside `AddAppForm` are the same as inside the old inline `AddAppModal`
- The "Add" / "Cancel" buttons exist in both forms
- Clicking the form's "Add" button still calls `onAdd` which calls `persist` which calls `set_setting`

Tests use `getByRole('button', { name: 'Add' })` or `screen.getByText('Add')` to find the submit button. Both still match.

**Step 5: Run lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 6: Commit**

```bash
git add app/src/windows/settings/ClipboardSettings.tsx app/src/windows/settings/ClipboardSettings.test.tsx
git commit -m "refactor(settings): polish ClipboardSettings — shared Modal + design-system header (closes #95)"
```

---

## Task 2: Final verification

**Files:** None modified. Verification only.

**Step 1: Full TS suite**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm -r --filter "@snk/*" --filter @snk/app test
```

Expected: 265 tests still passing (no test count change — same 6 component + 10 readEntries tests).

**Step 2: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 3: Vite build**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-clipboard-polish
pnpm --filter @snk/app build
```

Expected: succeeds.

**Step 4: Hand off** — report all-green to spec-auditor + quality-sentinel.

---

## Self-review notes

1. **Spec coverage:** #95 item 1 (design-system unification) → `h2` styling normalized to `font-display text-sm mb-3`. #95 item 2 (modal keyboard shortcuts) → both modals migrated to shared Modal, which provides Esc + focus trap. AddAppForm's `<form onSubmit>` provides Enter-to-submit. Acceptance: 6 existing component tests + 10 readEntries tests still pass.
2. **Placeholders:** none.
3. **Naming consistency:** `AddAppForm` and `ConfirmFrontmostBody` replace `AddAppModal` and `ConfirmFrontmostModal` — they're now form/body content, not the full modal chrome.
4. **Buildability:** Single implementer; one refactor file + one test file edit; no shared infrastructure changes.

## Plan-as-source-of-truth reminder

Real bugs → SendMessage `team-lead` BEFORE applying. Per memory `[[feedback_plan_as_source_of_truth]]`.
