# React work cluster — shared infra + per-feature panels

**Date:** 2026-05-25
**Status:** Approved
**Scope:** GitHub issues #36, #62, #87, #78, #95
**Closed during brainstorm:** #66 (already structurally complete — `app/src/windows/annotate/shapes/` exists)

## Problem

Five open issues in the React/TS layer have captured-decision approval but no design depth:

- **#36** — About panel: version + paths + updater status + OCR engine + diagnostics
- **#62** — Resurrect `@snk/ocr` and `@snk/updater` TS packages (TS bindings for the matching Rust plugins)
- **#87** — Theme picker step in the FirstRunWizard
- **#78** — "Hide own windows during capture" — React-side settings toggle
- **#95** — ClipboardSettings polish — design-system unification + modal keyboard handling

The React layer today has **no shared component directory**. `SettingsWindow.tsx` (351 LoC) has inline `SettingRow` and `Toggle` components. `ClipboardSettings.tsx` has inline `AddAppModal` and `ConfirmFrontmostModal` with no Esc/Enter handling and no focus trap. The cluster will introduce a new window (About), a new wizard step (theme picker), and one new settings toggle (hide-own-windows) — each of which would, without shared primitives, reinvent modal handling and settings-section layout.

## Goals

1. Establish a shared component foundation (`Modal`, `SettingsSection`, `SettingRow`, `Toggle`, `Button`) that all current and future React surfaces consume.
2. Ship the About panel with diagnostic-first layout, including a "Check Now" updater control that surfaces inline results and offers a one-click Install.
3. Resurrect the `@snk/ocr` and `@snk/updater` TS packages to back the About panel's status surfaces.
4. Add a theme picker step to the FirstRunWizard so first-launch users pick a theme before reaching the main window.
5. Add a single "hide own windows during capture" Settings toggle (React side; the Rust capture orchestrator already plumbs the value).
6. Polish ClipboardSettings to use the shared primitives — Esc/Enter on its two modals, normalized spacing and typography.

## Non-goals

- **Encryption section in About** — waits on #60 (SQLCipher). About will not mention encryption yet.
- **"Re-run OCR" button in About** — About panel stays read-only diagnostic. A per-capture re-run lives on the library context menu (separate issue).
- **Marketing hero in About** — diagnostic-first; no logo/tagline. snapper-keeper is a personal utility, not a product needing a marketing About box.
- **Splitting `AnnotateCanvas.tsx`** — #66 is already structurally complete via `app/src/windows/annotate/shapes/` and was closed during this brainstorm. Remaining LoC in `AnnotateCanvas.tsx` is canvas-level orchestration (Konva Stage, transformers, text-edit overlay, container sizing, crop region renderer), not per-shape code.
- **Rust-side changes for hide-own-windows** — the capture orchestrator already accepts the value; this design only covers the React toggle and persistence.

## Approach

### Architectural decision: shared infrastructure first

A single small PR introduces `app/src/components/` with the primitives. All other PRs in this cluster consume them. The alternative — building features ad-hoc and extracting shared components in a later polish pass — would ship the About panel using inline modals + bespoke styling, then force a refactor across About + ClipboardSettings together. Doing the extraction first is cheaper in total work and produces consistent visuals from day 1.

### Sequencing

| PR | Issue(s) | Scope | Estimated size |
|----|----------|-------|----------------|
| A | (foundation) | Shared `components/` directory; refactor `SettingsWindow.tsx` to consume | ~half day |
| B | #36 + #62 | About window + resurrect `@snk/ocr` + `@snk/updater` TS packages | ~1 day |
| C | #87 | Theme picker step in FirstRunWizard | 2-4 hours |
| D | #78 | Hide-own-windows Toggle in Settings | 2-4 hours |
| E | #95 | ClipboardSettings polish | 2-4 hours |

PR A is the keystone. PRs B-E depend on it. After A+B land, PRs C/D/E are independent and can be parallelized.

## Detailed design

### PR A — Shared component infrastructure

**New directory:** `app/src/components/`

```
app/src/components/
├── Modal/
│   ├── Modal.tsx          # rendered dialog UI
│   ├── ModalProvider.tsx  # context provider, owns modal state + portal
│   ├── useModal.ts        # imperative hook
│   └── index.ts           # public exports
├── Button.tsx             # primary / secondary / danger variants
├── SettingsSection.tsx    # header + body container with consistent spacing
├── SettingRow.tsx         # label + control + optional description
├── Toggle.tsx             # accessible switch
└── __tests__/             # vitest + @testing-library/react
    ├── Modal.test.tsx
    ├── SettingRow.test.tsx
    └── useModal.test.ts
```

**Modal API (imperative, hook-driven):**

```ts
type ConfirmOpts = {
  title: string;
  body: ReactNode;
  confirmLabel?: string;   // default: "Confirm"
  cancelLabel?: string;    // default: "Cancel"
  onConfirm: () => void | Promise<void>;
  danger?: boolean;        // styles primary button as destructive
};

type AlertOpts = {
  title: string;
  body: ReactNode;
  okLabel?: string;        // default: "OK"
};

type CustomOpts = {
  title: string;
  render: (ctx: { close: () => void }) => ReactNode;
};

type ModalAPI = {
  confirm(opts: ConfirmOpts): void;
  alert(opts: AlertOpts): void;
  custom(opts: CustomOpts): void;
};

const modal = useModal();
modal.confirm({
  title: "Delete capture?",
  body: "This cannot be undone.",
  danger: true,
  onConfirm: () => deleteCapture(id),
});
```

**Modal behavior:**

- Renders into a portal at `<div id="modal-root">` (added to `app/index.html`).
- Focus trap: while the modal is open, focus cycles within it; sibling content gets `inert` attribute.
- Esc closes the modal. For `.confirm`/`.alert`, this is equivalent to clicking the secondary/OK button.
- Enter triggers the primary action on `.confirm` and `.alert` only. `.custom` owns its own keyboard handling (forms inside `.custom` decide whether Enter submits).
- On close, focus returns to the element that opened the modal.
- Only one modal at a time. Calling `.confirm`/`.alert`/`.custom` while a modal is open replaces it.

**SettingsSection / SettingRow / Toggle:**

Extracted near-verbatim from `app/src/windows/settings/SettingsWindow.tsx`. The current inline implementations are well-shaped; this PR moves them into shared files and removes duplication. No visual changes to the existing settings window.

**Button variants:** primary, secondary, danger. Used by Modal's footer and by individual panels.

**Wiring:**

- `app/src/main.tsx` wraps the app in `<ModalProvider>`.
- `app/index.html` adds `<div id="modal-root"></div>` as a sibling to the main `<div id="root">`.

**Tests:**

- `useModal.test.ts` — verifies `.confirm` opens modal, Enter triggers `onConfirm`, Esc closes, focus returns to invoker.
- `Modal.test.tsx` — verifies focus trap, sibling inert behavior, single-modal replacement.
- `SettingRow.test.tsx` — verifies label/description/control rendering, accessible name wiring.

### PR B — About panel (#36) + resurrect packages (#62)

**Window registration:**

- `app/src/windows/about/AboutWindow.tsx`
- `app/src/windows/about/index.html`
- `app/src/windows/about/main.tsx` (entry point)
- `app/src-tauri/tauri.conf.json` — register `about` window (closed by default, opened on demand)
- Tray menu gains "About snapper-keeper…" entry that calls `WebviewWindow.getByLabel('about') ?? new WebviewWindow('about', {...})`.

**Sections, in order:**

1. **Version**
   - App version from Tauri `getVersion()` (already exposed).
   - Git short SHA from a build-time env var (`__GIT_SHA__` injected by Vite). Falls back to `"dev"` if not set.

2. **Storage**
   - Data directory: result of `path.appDataDir()` from `@tauri-apps/api/path`.
   - Log directory: result of `path.appLogDir()`.
   - Each path has an "Open" button (calls `open()` from `@tauri-apps/plugin-shell` or equivalent existing helper).

3. **Updater**
   - Renders state from `@snk/updater`: last check timestamp (or "never"), update channel, last result ("up to date" / "vX.Y.Z available" / "error: …").
   - **"Check Now" button:**
     - Click → button enters loading state with spinner.
     - Calls updater check IPC (resurrect the `@snk/updater` binding).
     - Result renders inline:
       - "Up to date (vX.Y.Z)" — green check.
       - "Update available: vX.Y.Z" — secondary text + **"Install"** button.
       - "Check failed: <reason>" — red text, button re-enabled.
     - Clicking **Install** starts the download (existing Rust-side flow), then surfaces a "Restart to install" prompt via `modal.confirm`.
   - Network failures and "no update available" both render inline in the Updater section. No toast/notification spillover.

4. **OCR engine**
   - Engine name (e.g., "Tesseract") and version, sourced from `@snk/ocr`'s status command.
   - If `@snk/ocr` reports `engine_unavailable`, the section shows "OCR not installed" with a link to the relevant README section.

5. **Privacy / License**
   - Two text links: "Privacy" (opens `PRIVACY.md` on GitHub) and "License" (opens `LICENSE` on GitHub).
   - No legal copy inline.

**Layout:**

Single scrollable column. Each section uses `SettingsSection` (header + body). Path and status rows use a custom row style — not `SettingRow`, because they have a different shape (label + value + action button, not label + toggleable control). About-specific row component lives in `app/src/windows/about/AboutRow.tsx`.

**TS packages to resurrect:**

```
packages/snk-ocr/
├── package.json
├── tsconfig.json
└── src/
    └── index.ts     # exports: getStatus(), types

packages/snk-updater/
├── package.json
├── tsconfig.json
└── src/
    └── index.ts     # exports: checkForUpdate(), installUpdate(), getLastCheck(), types
```

Each package's `index.ts` wraps `invoke` calls to the matching Rust plugin's commands. If a needed Rust command doesn't yet exist (likely for `getLastCheck`), stub the TS binding with a `// TODO(#NN)` and have it return a sensible default ("never"). The About panel renders defaults gracefully so PR B can land without blocking on additional Rust work.

### PR C — Theme picker step in FirstRunWizard (#87)

**Files:**

- Create: `app/src/windows/first-run-wizard/steps/ThemePickerStep.tsx`
- Modify: `app/src/windows/first-run-wizard/FirstRunWizard.tsx` — insert as step 2 (after Welcome)

**Behavior:**

- Renders a small grid of radio-card options. Each card shows the theme name and a tiny preview swatch (background, accent, text).
- Themes sourced from the existing `lib/theme.ts` `THEME_FAMILIES` map.
- **Live preview:** selecting a card immediately applies the theme to the wizard chrome itself, so the user sees the change without leaving the step.
- "Next" persists the choice via the existing theme settings IPC command.

**No new settings storage.** The theme picker writes to the same setting the SettingsWindow theme picker writes to.

### PR D — Hide own windows during capture (#78)

**File:** Modify `app/src/windows/settings/SettingsWindow.tsx`.

**Change:** Add a `SettingRow` with a `Toggle` under the Capture section. Label: "Hide snapper-keeper windows during capture." Description: "Prevents the app's own windows from appearing in screen captures." Default value: `true`.

**Persistence:** Through the existing settings IPC. Rust side already plumbs `hide_own_windows_during_capture` through the capture orchestrator; this PR only adds the React-side control.

### PR E — ClipboardSettings polish (#95)

**File:** Modify `app/src/windows/settings/ClipboardSettings.tsx`.

**Changes:**

1. Replace inline `AddAppModal` with `modal.custom({ title: "Add app", render: ({close}) => <AddAppForm onSubmit={...} onCancel={close} /> })`. Move the form body to a new `AddAppForm` component. Now gets focus trap, Esc-to-close, and Enter-to-submit-form for free.
2. Replace inline `ConfirmFrontmostModal` with `modal.confirm({ title, body, onConfirm })`. Gets keyboard handling for free.
3. Swap inline section/row markup for `SettingsSection`/`SettingRow`/`Toggle`.
4. Normalize spacing and typography tokens to match `SettingsWindow.tsx`'s conventions (after PR A normalizes them).

**No behavior changes** beyond the keyboard handling on the modals. Existing IPC commands, validation, error messages preserved.

## Data flow

```
ModalProvider (app root)
  ├── React context: { state, setState }
  └── Portal: <div id="modal-root">

useModal()
  ├── reads context, returns { confirm, alert, custom }
  └── each method sets context state → Modal re-renders with new content

AboutWindow
  ├── useEffect(): load version, paths, updater state, OCR state on mount
  ├── "Check Now" → @snk/updater.checkForUpdate() → setState
  └── "Install" → modal.confirm("Restart to install?") → @snk/updater.installUpdate()

FirstRunWizard (Theme step)
  ├── radio-card change → setLocalTheme(t); applyThemeToDOM(t)  // live preview
  └── "Next" → invoke("settings_set_theme", { theme: localTheme }) → next step

SettingsWindow ("Hide own windows" toggle)
  └── toggle change → invoke("settings_set_hide_own_windows", { hide: bool })

ClipboardSettings ("Add app" button)
  └── click → modal.custom({ render: ({close}) => <AddAppForm onSubmit={v => { invoke(...); close(); }} /> })
```

## Error handling

- **Updater check failure:** Renders inline in Updater section ("Check failed: <reason>"). No toast.
- **Path resolution failure** (`appDataDir`/`appLogDir` rejects): Section shows "Unavailable" in place of the path. Logged via existing tracing setup.
- **OCR engine unavailable:** Section shows "OCR not installed" with link to README. Not treated as an error.
- **Modal `onConfirm` rejects:** Modal stays open; primary button leaves loading state; error surfaces via existing toast/notification mechanism (caller decides). Modal does not own error display for `.confirm` — callers handle it.

## Testing

- **PR A:** Unit tests for Modal (focus trap, Esc, Enter, single-modal replacement), SettingRow (label/description/a11y), useModal (imperative API).
- **PR B:** Component test for AboutWindow with mocked `@snk/updater` / `@snk/ocr` modules — verifies all 5 sections render, "Check Now" loading + result states, "Install" confirm flow.
- **PR C:** Component test for ThemePickerStep — verifies radio cards render for all themes, live preview applies on selection, "Next" calls the IPC with the selected theme.
- **PR D:** Component test for the new Toggle row — verifies default `true`, persists on change.
- **PR E:** Component tests for the migrated modals — verify Esc closes, Enter submits AddAppForm, ConfirmFrontmostModal preserves existing IPC contract.

No new E2E coverage required. The shared primitives are pure React; the per-feature panels are thin glue over existing IPC commands.

## Open questions

None blocking. Implementation may surface small decisions (e.g., exact Tauri command name for path resolution in PR B) — those go through plan-as-source-of-truth as usual.

## Implementation order

1. **PR A** — shared infrastructure. Land clean before anything else.
2. **PR B** — About panel + packages. Bundle because About needs `@snk/updater` bindings.
3. **PRs C / D / E** — can be sequential or parallel after A+B.
