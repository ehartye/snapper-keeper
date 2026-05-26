# React Shared Components (PR A) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:team-driven-development to implement this plan.

**Goal:** Establish `app/src/components/` with shared primitives (Button, Toggle, SettingRow, SettingsSection, Modal subsystem) so the rest of the React cluster (About panel, theme wizard, ClipboardSettings polish, hide-own-windows toggle) can build on a consistent foundation.

**Architecture:** Pure-React extraction with no runtime behavior changes for existing surfaces. Inline `SettingRow` and `Toggle` in `SettingsWindow.tsx` move to shared modules. New `SettingsSection` codifies the rounded-card row-group pattern. New `Modal` subsystem exposes an imperative `useModal()` hook with `.confirm` / `.alert` / `.custom` methods, focus trap (via sibling `inert`), Esc-to-close, Enter-to-primary, single-modal-at-a-time, and portal rendering into `<div id="modal-root">`.

**Tech Stack:** React 18, TypeScript (strict + noUncheckedIndexedAccess), Tailwind CSS, Vitest + @testing-library/react + happy-dom, @testing-library/user-event.

**Spec:** `docs/superpowers/specs/2026-05-25-react-cluster-design.md` (cherry-picked onto this branch).

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-shared-components/`
**Branch:** `feat/react-shared-components` (off `origin/main` 8ab1ac5)
**Baseline:** 173/173 TS tests passing on first install.

---

## Conventions

- **Commits:** Conventional Commits. All commits in this plan use `feat(ui):`, `test(ui):`, or `refactor(ui):` scopes.
- **Staging:** `git add <explicit-paths>`, NEVER `git add .` or `-A`. Per CLAUDE.md.
- **One task = one commit** unless a precursor is justified (none expected in PR A).
- **TDD:** Each task is red → green → refactor. Tests live alongside the code in `app/src/components/__tests__/`.
- **No comments unless the WHY is non-obvious** (e.g., pill dimensions with travel reasoning is a WHY; "renders the button" is not).

## Dependency graph (for team-driven scheduling)

```
T1 (Button)        ─┐
T2 (Toggle)        ─┤
T3 (SettingRow)    ─┼─→ T9 (refactor SettingsWindow) → T10 (verify)
T4 (SettingsSec)   ─┤
                    │
T5 (ModalProvider) → T6 (Modal render) → T7 (useModal) → T8 (wire main.tsx) ─┘
```

T1-T5 are independent and can run in parallel. T6 needs T5. T7 needs T6. T8 needs T7. T9 needs T2, T3, T4. T10 needs everything.

---

## Task 1: Button component

**Files:**
- Create: `app/src/components/Button.tsx`
- Create: `app/src/components/__tests__/Button.test.tsx`

**Step 1: Write the failing test**

Create `app/src/components/__tests__/Button.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from '../Button';

describe('<Button />', () => {
  it('renders children as label', () => {
    render(<Button onClick={() => {}}>Save</Button>);
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>OK</Button>);
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('applies primary styling by default', () => {
    render(<Button onClick={() => {}}>P</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-primary');
  });

  it('applies secondary styling when variant="secondary"', () => {
    render(<Button variant="secondary" onClick={() => {}}>S</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-surface-2');
  });

  it('applies danger styling when variant="danger"', () => {
    render(<Button variant="danger" onClick={() => {}}>D</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-red-600');
  });

  it('is disabled when disabled prop is true', () => {
    render(<Button disabled onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('does not call onClick when disabled', () => {
    const onClick = vi.fn();
    render(<Button disabled onClick={onClick}>X</Button>);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).not.toHaveBeenCalled();
  });

  it('forwards type prop (default "button")', () => {
    const { rerender } = render(<Button onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'button');
    rerender(<Button type="submit" onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'submit');
  });
});
```

**Step 2: Run test, verify it fails**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-react-shared-components
pnpm --filter @snk/app test -- --run src/components/__tests__/Button.test.tsx
```

Expected: FAIL with `Cannot find module '../Button'`.

**Step 3: Implement Button**

Create `app/src/components/Button.tsx`:

```tsx
import type { ButtonHTMLAttributes, ReactNode } from 'react';

type Variant = 'primary' | 'secondary' | 'danger';

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  children: ReactNode;
  variant?: Variant;
}

const variantClasses: Record<Variant, string> = {
  primary: 'bg-primary text-bg hover:brightness-110',
  secondary: 'bg-surface-2 text-fg border border-border hover:bg-surface',
  danger: 'bg-red-600 text-white hover:bg-red-700',
};

export function Button({
  children,
  variant = 'primary',
  type = 'button',
  className = '',
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={`px-3 py-1.5 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${variantClasses[variant]} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/Button.test.tsx
```

Expected: 8 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/Button.tsx app/src/components/__tests__/Button.test.tsx
git commit -m "feat(ui): add Button component with primary/secondary/danger variants"
```

---

## Task 2: Toggle component (extracted)

**Files:**
- Create: `app/src/components/Toggle.tsx`
- Create: `app/src/components/__tests__/Toggle.test.tsx`

**Step 1: Write the failing test**

Create `app/src/components/__tests__/Toggle.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Toggle } from '../Toggle';

describe('<Toggle />', () => {
  it('renders as a switch', () => {
    render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toBeInTheDocument();
  });

  it('reports the current state via aria-checked', () => {
    const { rerender } = render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
    rerender(<Toggle value={true} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'true');
  });

  it('calls onChange with the inverted value when clicked', () => {
    const onChange = vi.fn();
    const { rerender } = render(<Toggle value={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenCalledWith(true);

    onChange.mockClear();
    rerender(<Toggle value={true} onChange={onChange} />);
    fireEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('applies the "on" style when value is true', () => {
    render(<Toggle value={true} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveClass('bg-primary');
  });

  it('applies the "off" style when value is false', () => {
    render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveClass('bg-surface-2');
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/Toggle.test.tsx
```

Expected: FAIL with `Cannot find module '../Toggle'`.

**Step 3: Implement Toggle**

Create `app/src/components/Toggle.tsx` (extracted from `SettingsWindow.tsx:36-54`, preserving the pill-dimensions comment because the geometry is non-obvious):

```tsx
interface ToggleProps {
  value: boolean;
  onChange: (next: boolean) => void;
}

// Pill: 44×22 (w-11 h-[22px] — wider so the thumb has clean travel).
// Thumb: 16×16 (w-4 h-4). Off: thumb at x=2; on: x=(100% - 18px).
export function Toggle({ value, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      className={`w-11 h-[22px] rounded-full relative transition-colors border border-border ${
        value ? 'bg-primary' : 'bg-surface-2'
      }`}
      onClick={() => onChange(!value)}
    >
      <span
        className="absolute top-[2px] w-4 h-4 rounded-full bg-bg transition-[left] duration-150"
        style={{ left: value ? 'calc(100% - 18px)' : '2px' }}
      />
    </button>
  );
}
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/Toggle.test.tsx
```

Expected: 5 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/Toggle.tsx app/src/components/__tests__/Toggle.test.tsx
git commit -m "feat(ui): extract Toggle into shared components/"
```

---

## Task 3: SettingRow component (extracted)

**Files:**
- Create: `app/src/components/SettingRow.tsx`
- Create: `app/src/components/__tests__/SettingRow.test.tsx`

**Step 1: Write the failing test**

Create `app/src/components/__tests__/SettingRow.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingRow } from '../SettingRow';

describe('<SettingRow />', () => {
  it('renders the label', () => {
    render(
      <SettingRow label="History size">
        <input />
      </SettingRow>,
    );
    expect(screen.getByText('History size')).toBeInTheDocument();
  });

  it('renders the description when provided', () => {
    render(
      <SettingRow label="Auto-copy" description="Copy capture to clipboard">
        <input />
      </SettingRow>,
    );
    expect(screen.getByText('Copy capture to clipboard')).toBeInTheDocument();
  });

  it('does not render a description element when description is omitted', () => {
    const { container } = render(
      <SettingRow label="History size">
        <input data-testid="control" />
      </SettingRow>,
    );
    // The label div + the control div = 2 children of the inner wrapper.
    // No description div should be present.
    expect(container.querySelectorAll('div.text-\\[11px\\]')).toHaveLength(0);
  });

  it('renders the control children', () => {
    render(
      <SettingRow label="X">
        <input data-testid="control" />
      </SettingRow>,
    );
    expect(screen.getByTestId('control')).toBeInTheDocument();
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/SettingRow.test.tsx
```

Expected: FAIL with `Cannot find module '../SettingRow'`.

**Step 3: Implement SettingRow**

Create `app/src/components/SettingRow.tsx` (extracted from `SettingsWindow.tsx:16-34`):

```tsx
import type { ReactNode } from 'react';

interface SettingRowProps {
  label: string;
  description?: string;
  children: ReactNode;
}

export function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-3">
      <div className="min-w-0">
        <div className="text-sm text-fg">{label}</div>
        {description && (
          <div className="text-[11px] text-fg-muted mt-0.5">{description}</div>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/SettingRow.test.tsx
```

Expected: 4 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/SettingRow.tsx app/src/components/__tests__/SettingRow.test.tsx
git commit -m "feat(ui): extract SettingRow into shared components/"
```

---

## Task 4: SettingsSection component (new)

**Files:**
- Create: `app/src/components/SettingsSection.tsx`
- Create: `app/src/components/__tests__/SettingsSection.test.tsx`

**Context:** The recurring pattern in `SettingsWindow.tsx` is `<section><h2>title</h2><div className="bg-surface rounded-xl ...">rows</div></section>`. `SettingsSection` codifies it so callers write `<SettingsSection title="Capture"><SettingRow…/>…</SettingsSection>`. The Appearance section (grid-based, not row-group) does NOT use SettingsSection — it stays inline.

**Step 1: Write the failing test**

Create `app/src/components/__tests__/SettingsSection.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingsSection } from '../SettingsSection';

describe('<SettingsSection />', () => {
  it('renders the title in an h2', () => {
    render(
      <SettingsSection title="Capture">
        <div>row</div>
      </SettingsSection>,
    );
    const heading = screen.getByRole('heading', { name: 'Capture', level: 2 });
    expect(heading).toBeInTheDocument();
  });

  it('renders children inside the card body', () => {
    render(
      <SettingsSection title="Capture">
        <div data-testid="child">child</div>
      </SettingsSection>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('applies the card classes to the body wrapper', () => {
    const { container } = render(
      <SettingsSection title="Capture">
        <div>row</div>
      </SettingsSection>,
    );
    const body = container.querySelector('.bg-surface');
    expect(body).not.toBeNull();
    expect(body).toHaveClass('rounded-xl');
    expect(body).toHaveClass('border');
    expect(body).toHaveClass('divide-y');
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/SettingsSection.test.tsx
```

Expected: FAIL with `Cannot find module '../SettingsSection'`.

**Step 3: Implement SettingsSection**

Create `app/src/components/SettingsSection.tsx`:

```tsx
import type { ReactNode } from 'react';

interface SettingsSectionProps {
  title: string;
  children: ReactNode;
}

export function SettingsSection({ title, children }: SettingsSectionProps) {
  return (
    <section>
      <h2 className="font-display text-sm mb-3">{title}</h2>
      <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
        {children}
      </div>
    </section>
  );
}
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/SettingsSection.test.tsx
```

Expected: 3 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/SettingsSection.tsx app/src/components/__tests__/SettingsSection.test.tsx
git commit -m "feat(ui): add SettingsSection (title + card-body wrapper)"
```

---

## Task 5: ModalProvider + types (context shell)

**Files:**
- Create: `app/src/components/Modal/types.ts`
- Create: `app/src/components/Modal/ModalProvider.tsx`
- Create: `app/src/components/Modal/index.ts`
- Create: `app/src/components/__tests__/ModalProvider.test.tsx`

**Context:** ModalProvider owns modal state. It exposes the React context that `useModal` consumes. It does NOT yet render the modal UI — that's Task 6. This task establishes the types and context plumbing only.

**Step 1: Write the failing test**

Create `app/src/components/__tests__/ModalProvider.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, renderHook, act } from '@testing-library/react';
import { ModalProvider } from '../Modal/ModalProvider';
import { useModalContext } from '../Modal/ModalProvider';

describe('<ModalProvider />', () => {
  it('renders children', () => {
    render(
      <ModalProvider>
        <div data-testid="child">child</div>
      </ModalProvider>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('provides a context whose default modal state is null', () => {
    const { result } = renderHook(() => useModalContext(), {
      wrapper: ({ children }) => <ModalProvider>{children}</ModalProvider>,
    });
    expect(result.current.modal).toBeNull();
  });

  it('setModal replaces the current modal state', () => {
    const { result } = renderHook(() => useModalContext(), {
      wrapper: ({ children }) => <ModalProvider>{children}</ModalProvider>,
    });
    act(() => {
      result.current.setModal({
        kind: 'alert',
        title: 'Hi',
        body: 'Hello',
        okLabel: 'OK',
      });
    });
    expect(result.current.modal).toEqual({
      kind: 'alert',
      title: 'Hi',
      body: 'Hello',
      okLabel: 'OK',
    });
  });

  it('throws when useModalContext is used outside ModalProvider', () => {
    // Silence the React error logged by renderHook.
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => renderHook(() => useModalContext())).toThrow(/ModalProvider/);
    spy.mockRestore();
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/ModalProvider.test.tsx
```

Expected: FAIL with `Cannot find module '../Modal/ModalProvider'`.

**Step 3: Implement types + provider**

Create `app/src/components/Modal/types.ts`:

```ts
import type { ReactNode } from 'react';

export interface ConfirmOpts {
  title: string;
  body: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void | Promise<void>;
  danger?: boolean;
}

export interface AlertOpts {
  title: string;
  body: ReactNode;
  okLabel?: string;
}

export interface CustomOpts {
  title: string;
  render: (ctx: { close: () => void }) => ReactNode;
}

export type ModalState =
  | (ConfirmOpts & { kind: 'confirm' })
  | (AlertOpts & { kind: 'alert' })
  | (CustomOpts & { kind: 'custom' });
```

Create `app/src/components/Modal/ModalProvider.tsx`:

```tsx
import { createContext, useContext, useState, type ReactNode } from 'react';
import type { ModalState } from './types';

interface ModalContextValue {
  modal: ModalState | null;
  setModal: (next: ModalState | null) => void;
}

const ModalContext = createContext<ModalContextValue | null>(null);

export function ModalProvider({ children }: { children: ReactNode }) {
  const [modal, setModal] = useState<ModalState | null>(null);
  return (
    <ModalContext.Provider value={{ modal, setModal }}>
      {children}
    </ModalContext.Provider>
  );
}

export function useModalContext(): ModalContextValue {
  const ctx = useContext(ModalContext);
  if (!ctx) {
    throw new Error('useModalContext must be used inside a <ModalProvider>');
  }
  return ctx;
}
```

Create `app/src/components/Modal/index.ts`:

```ts
export { ModalProvider } from './ModalProvider';
export type { ConfirmOpts, AlertOpts, CustomOpts, ModalState } from './types';
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/ModalProvider.test.tsx
```

Expected: 4 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/Modal/types.ts app/src/components/Modal/ModalProvider.tsx app/src/components/Modal/index.ts app/src/components/__tests__/ModalProvider.test.tsx
git commit -m "feat(ui): add Modal context + types (provider shell)"
```

---

## Task 6: Modal rendering (portal + keyboard + focus trap)

**Files:**
- Create: `app/src/components/Modal/Modal.tsx`
- Modify: `app/src/components/Modal/index.ts` (re-export Modal)
- Create: `app/src/components/__tests__/Modal.test.tsx`
- Modify: `app/src/components/Modal/ModalProvider.tsx` (render `<Modal />` inside the provider)

**Context:** Modal reads from `useModalContext()`, renders into a portal (`<div id="modal-root">`), handles Esc + Enter, manages focus trap via `inert` on siblings, and returns focus on close. Single-modal-at-a-time is enforced by the provider's `modal` being a single value.

**Step 1: Write the failing test**

Create `app/src/components/__tests__/Modal.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ModalProvider, useModalContext } from '../Modal/ModalProvider';
import type { ModalState } from '../Modal/types';

function SetModalOnMount({ state }: { state: ModalState }) {
  const { setModal } = useModalContext();
  // Open the modal once on mount.
  if (state) setTimeout(() => setModal(state), 0);
  return null;
}

beforeEach(() => {
  // Ensure modal-root exists in the test DOM.
  const existing = document.getElementById('modal-root');
  if (existing) existing.remove();
  const root = document.createElement('div');
  root.id = 'modal-root';
  document.body.appendChild(root);
});

describe('<Modal />', () => {
  it('renders nothing when no modal is open', () => {
    render(
      <ModalProvider>
        <div>app content</div>
      </ModalProvider>,
    );
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('renders a dialog with title and body when an alert is open', async () => {
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{ kind: 'alert', title: 'Hello', body: 'World', okLabel: 'OK' }}
        />
      </ModalProvider>,
    );
    await screen.findByRole('dialog');
    expect(screen.getByText('Hello')).toBeInTheDocument();
    expect(screen.getByText('World')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'OK' })).toBeInTheDocument();
  });

  it('clicking OK on an alert closes the modal', async () => {
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{ kind: 'alert', title: 'Hi', body: 'Bye', okLabel: 'OK' }}
        />
      </ModalProvider>,
    );
    const ok = await screen.findByRole('button', { name: 'OK' });
    fireEvent.click(ok);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Esc closes the modal', async () => {
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{ kind: 'alert', title: 'Hi', body: 'Bye', okLabel: 'OK' }}
        />
      </ModalProvider>,
    );
    await screen.findByRole('dialog');
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('renders confirm with Confirm + Cancel buttons and Enter triggers onConfirm', async () => {
    const onConfirm = vi.fn();
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{
            kind: 'confirm',
            title: 'Delete?',
            body: 'Are you sure?',
            confirmLabel: 'Delete',
            cancelLabel: 'Cancel',
            onConfirm,
          }}
        />
      </ModalProvider>,
    );
    await screen.findByRole('dialog');
    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    fireEvent.keyDown(document.body, { key: 'Enter' });
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('confirm Cancel does NOT call onConfirm', async () => {
    const onConfirm = vi.fn();
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{
            kind: 'confirm',
            title: 'Delete?',
            body: '',
            confirmLabel: 'Delete',
            cancelLabel: 'Cancel',
            onConfirm,
          }}
        />
      </ModalProvider>,
    );
    const cancel = await screen.findByRole('button', { name: 'Cancel' });
    fireEvent.click(cancel);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('confirm with danger=true marks the confirm button as danger styled', async () => {
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{
            kind: 'confirm',
            title: 'Delete?',
            body: '',
            confirmLabel: 'Delete',
            onConfirm: () => {},
            danger: true,
          }}
        />
      </ModalProvider>,
    );
    const del = await screen.findByRole('button', { name: 'Delete' });
    expect(del).toHaveClass('bg-red-600');
  });

  it('custom modals render the provided ReactNode and Enter does NOT auto-submit', async () => {
    const onFormSubmit = vi.fn((e) => e.preventDefault());
    render(
      <ModalProvider>
        <SetModalOnMount
          state={{
            kind: 'custom',
            title: 'Form',
            render: ({ close }) => (
              <form onSubmit={onFormSubmit} data-testid="custom-form">
                <input data-testid="input" />
                <button type="submit">Submit</button>
                <button type="button" onClick={close}>
                  Close
                </button>
              </form>
            ),
          }}
        />
      </ModalProvider>,
    );
    await screen.findByRole('dialog');
    expect(screen.getByTestId('custom-form')).toBeInTheDocument();
    // The global Enter handler must NOT fire for .custom — the form
    // owns its own submission semantics.
    fireEvent.keyDown(document.body, { key: 'Enter' });
    expect(onFormSubmit).not.toHaveBeenCalled();
  });

  it('opening a second modal replaces the first', async () => {
    function Opener() {
      const { setModal } = useModalContext();
      return (
        <div>
          <button
            onClick={() =>
              setModal({ kind: 'alert', title: 'First', body: '', okLabel: 'OK' })
            }
          >
            open-first
          </button>
          <button
            onClick={() =>
              setModal({ kind: 'alert', title: 'Second', body: '', okLabel: 'OK' })
            }
          >
            open-second
          </button>
        </div>
      );
    }
    render(
      <ModalProvider>
        <Opener />
      </ModalProvider>,
    );
    fireEvent.click(screen.getByText('open-first'));
    expect(screen.getByText('First')).toBeInTheDocument();
    fireEvent.click(screen.getByText('open-second'));
    expect(screen.queryByText('First')).toBeNull();
    expect(screen.getByText('Second')).toBeInTheDocument();
  });

  it('marks sibling content as inert while a modal is open', async () => {
    function Layout() {
      const { setModal } = useModalContext();
      return (
        <div>
          <div data-testid="sibling">app body</div>
          <button
            onClick={() =>
              setModal({ kind: 'alert', title: 'Hi', body: '', okLabel: 'OK' })
            }
          >
            open
          </button>
        </div>
      );
    }
    render(
      <ModalProvider>
        <Layout />
      </ModalProvider>,
    );
    fireEvent.click(screen.getByText('open'));
    await screen.findByRole('dialog');
    // The provider's children wrapper should have inert set while modal open.
    const wrapper = document.getElementById('modal-app-content');
    expect(wrapper?.hasAttribute('inert')).toBe(true);
    // Close and verify inert is removed.
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(wrapper?.hasAttribute('inert')).toBe(false);
  });

  it('returns focus to the invoking element on close', async () => {
    function Opener() {
      const { setModal } = useModalContext();
      return (
        <button
          data-testid="opener"
          onClick={() =>
            setModal({ kind: 'alert', title: 'Hi', body: '', okLabel: 'OK' })
          }
        >
          open
        </button>
      );
    }
    render(
      <ModalProvider>
        <Opener />
      </ModalProvider>,
    );
    const opener = screen.getByTestId('opener');
    act(() => opener.focus());
    expect(document.activeElement).toBe(opener);
    fireEvent.click(opener);
    await screen.findByRole('dialog');
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(document.activeElement).toBe(opener);
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/Modal.test.tsx
```

Expected: FAIL — `dialog` role not found because Modal component doesn't exist yet.

**Step 3: Implement Modal**

Create `app/src/components/Modal/Modal.tsx`:

```tsx
import { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { Button } from '../Button';
import { useModalContext } from './ModalProvider';

export function Modal() {
  const { modal, setModal } = useModalContext();
  const previouslyFocused = useRef<HTMLElement | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  const close = () => setModal(null);

  // Snapshot the focused element on open so we can restore it on close.
  useEffect(() => {
    if (modal) {
      previouslyFocused.current = document.activeElement as HTMLElement | null;
      // Focus first focusable in dialog after render.
      requestAnimationFrame(() => {
        dialogRef.current?.querySelector<HTMLElement>('button, input, textarea, [tabindex]')?.focus();
      });
    } else if (previouslyFocused.current) {
      previouslyFocused.current.focus();
      previouslyFocused.current = null;
    }
  }, [modal]);

  // Global key handling — Esc and (for confirm/alert) Enter.
  // Local `close` inside the effect avoids referencing the outer `close`
  // (whose identity changes each render) in the deps list.
  useEffect(() => {
    if (!modal) return;
    const closeNow = () => setModal(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeNow();
      } else if (e.key === 'Enter') {
        if (modal.kind === 'confirm') {
          e.preventDefault();
          // Synchronous so test assertions about post-Enter DOM state
          // observe the closed modal in the same tick.
          modal.onConfirm();
          closeNow();
        } else if (modal.kind === 'alert') {
          e.preventDefault();
          closeNow();
        }
        // .custom: ignore Enter — caller's form owns it.
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [modal, setModal]);

  if (!modal) return null;

  const root = document.getElementById('modal-root');
  if (!root) return null;

  const renderBody = () => {
    if (modal.kind === 'custom') return modal.render({ close });
    return <div className="text-sm text-fg">{modal.body}</div>;
  };

  const renderFooter = () => {
    if (modal.kind === 'alert') {
      return (
        <Button onClick={close} variant="primary">
          {modal.okLabel ?? 'OK'}
        </Button>
      );
    }
    if (modal.kind === 'confirm') {
      const m = modal;
      return (
        <>
          <Button variant="secondary" onClick={close}>
            {m.cancelLabel ?? 'Cancel'}
          </Button>
          <Button
            variant={m.danger ? 'danger' : 'primary'}
            onClick={() => {
              m.onConfirm();
              close();
            }}
          >
            {m.confirmLabel ?? 'Confirm'}
          </Button>
        </>
      );
    }
    return null;
  };

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={close}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={modal.title}
        className="bg-bg border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-display text-base mb-3">{modal.title}</h3>
        <div className="mb-5">{renderBody()}</div>
        <div className="flex justify-end gap-2">{renderFooter()}</div>
      </div>
    </div>,
    root,
  );
}
```

Modify `app/src/components/Modal/ModalProvider.tsx` to render `<Modal />` plus wrap children in an inert-controlled element:

Replace the existing return in `ModalProvider`:

```tsx
import { createContext, useContext, useState, type ReactNode } from 'react';
import type { ModalState } from './types';
import { Modal } from './Modal';

interface ModalContextValue {
  modal: ModalState | null;
  setModal: (next: ModalState | null) => void;
}

const ModalContext = createContext<ModalContextValue | null>(null);

export function ModalProvider({ children }: { children: ReactNode }) {
  const [modal, setModal] = useState<ModalState | null>(null);
  // `inert` on the app content while modal is open keeps focus + clicks
  // out of the background. React doesn't yet type `inert` natively on
  // intrinsic elements (it lands in React 19), so we conditionally spread.
  const inertProp = modal ? { inert: '' as unknown as undefined } : {};
  return (
    <ModalContext.Provider value={{ modal, setModal }}>
      <div id="modal-app-content" {...inertProp}>
        {children}
      </div>
      <Modal />
    </ModalContext.Provider>
  );
}

export function useModalContext(): ModalContextValue {
  const ctx = useContext(ModalContext);
  if (!ctx) {
    throw new Error('useModalContext must be used inside a <ModalProvider>');
  }
  return ctx;
}
```

Modify `app/src/components/Modal/index.ts`:

```ts
export { ModalProvider } from './ModalProvider';
export { Modal } from './Modal';
export type { ConfirmOpts, AlertOpts, CustomOpts, ModalState } from './types';
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/Modal.test.tsx src/components/__tests__/ModalProvider.test.tsx
```

Expected: all tests passing (4 ModalProvider + 11 Modal = 15).

**Step 5: Commit**

```bash
git add app/src/components/Modal/Modal.tsx app/src/components/Modal/ModalProvider.tsx app/src/components/Modal/index.ts app/src/components/__tests__/Modal.test.tsx
git commit -m "feat(ui): render Modal with portal, keyboard, focus trap, and inert siblings"
```

---

## Task 7: useModal hook

**Files:**
- Create: `app/src/components/Modal/useModal.ts`
- Modify: `app/src/components/Modal/index.ts` (re-export `useModal`)
- Create: `app/src/components/__tests__/useModal.test.tsx`

**Step 1: Write the failing test**

Create `app/src/components/__tests__/useModal.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ModalProvider } from '../Modal/ModalProvider';
import { useModal } from '../Modal/useModal';

beforeEach(() => {
  const existing = document.getElementById('modal-root');
  if (existing) existing.remove();
  const root = document.createElement('div');
  root.id = 'modal-root';
  document.body.appendChild(root);
});

function ConfirmOpener({ onConfirm }: { onConfirm: () => void }) {
  const modal = useModal();
  return (
    <button
      onClick={() =>
        modal.confirm({
          title: 'Delete?',
          body: 'Sure?',
          confirmLabel: 'Yes',
          cancelLabel: 'No',
          onConfirm,
        })
      }
    >
      open-confirm
    </button>
  );
}

function AlertOpener() {
  const modal = useModal();
  return (
    <button
      onClick={() => modal.alert({ title: 'Hi', body: 'There', okLabel: 'OK' })}
    >
      open-alert
    </button>
  );
}

function CustomOpener() {
  const modal = useModal();
  return (
    <button
      onClick={() =>
        modal.custom({
          title: 'Form',
          render: ({ close }) => (
            <div>
              <span>custom-body</span>
              <button onClick={close}>close-custom</button>
            </div>
          ),
        })
      }
    >
      open-custom
    </button>
  );
}

describe('useModal()', () => {
  it('confirm: opens a confirm dialog and clicking primary calls onConfirm + closes', () => {
    const onConfirm = vi.fn();
    render(
      <ModalProvider>
        <ConfirmOpener onConfirm={onConfirm} />
      </ModalProvider>,
    );
    fireEvent.click(screen.getByText('open-confirm'));
    expect(screen.getByText('Delete?')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Yes' }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('alert: opens an alert dialog and OK closes it', () => {
    render(
      <ModalProvider>
        <AlertOpener />
      </ModalProvider>,
    );
    fireEvent.click(screen.getByText('open-alert'));
    expect(screen.getByText('Hi')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('custom: render-prop receives a close fn that closes the modal', () => {
    render(
      <ModalProvider>
        <CustomOpener />
      </ModalProvider>,
    );
    fireEvent.click(screen.getByText('open-custom'));
    expect(screen.getByText('custom-body')).toBeInTheDocument();
    fireEvent.click(screen.getByText('close-custom'));
    expect(screen.queryByText('custom-body')).toBeNull();
  });

  it('throws a helpful error when used outside ModalProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => render(<ConfirmOpener onConfirm={() => {}} />)).toThrow(/ModalProvider/);
    spy.mockRestore();
  });
});
```

**Step 2: Run test, verify it fails**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/useModal.test.tsx
```

Expected: FAIL with `Cannot find module '../Modal/useModal'`.

**Step 3: Implement useModal**

Create `app/src/components/Modal/useModal.ts`:

```ts
import { useMemo } from 'react';
import { useModalContext } from './ModalProvider';
import type { ConfirmOpts, AlertOpts, CustomOpts } from './types';

export interface ModalAPI {
  confirm: (opts: ConfirmOpts) => void;
  alert: (opts: AlertOpts) => void;
  custom: (opts: CustomOpts) => void;
}

export function useModal(): ModalAPI {
  const { setModal } = useModalContext();
  // Memoize so consumers can depend on the api in useEffect deps.
  return useMemo(
    () => ({
      confirm: (opts) => setModal({ kind: 'confirm', ...opts }),
      alert: (opts) => setModal({ kind: 'alert', ...opts }),
      custom: (opts) => setModal({ kind: 'custom', ...opts }),
    }),
    [setModal],
  );
}
```

Modify `app/src/components/Modal/index.ts`:

```ts
export { ModalProvider } from './ModalProvider';
export { Modal } from './Modal';
export { useModal } from './useModal';
export type { ConfirmOpts, AlertOpts, CustomOpts, ModalState } from './types';
export type { ModalAPI } from './useModal';
```

**Step 4: Run test, verify it passes**

```bash
pnpm --filter @snk/app test -- --run src/components/__tests__/useModal.test.tsx
```

Expected: 4 tests passing.

**Step 5: Commit**

```bash
git add app/src/components/Modal/useModal.ts app/src/components/Modal/index.ts app/src/components/__tests__/useModal.test.tsx
git commit -m "feat(ui): add useModal() imperative hook"
```

---

## Task 8: Wire ModalProvider in main.tsx + add modal-root to index.html

**Files:**
- Modify: `app/index.html` (add `<div id="modal-root">` after `<div id="root">`)
- Modify: `app/src/main.tsx` (wrap `<App />` with `<ModalProvider>`)

**Context:** No new tests here — the existing `SettingsWindow.test.tsx` and other window tests verify that `<App />` still renders correctly under the new provider. The Modal tests already cover provider behavior.

**Step 1: Modify `app/index.html`**

Replace the current body contents:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>snapper-keeper</title>
    <!-- Fonts are bundled via @fontsource (see src/lib/fonts.ts) — no CDN. -->
  </head>
  <body>
    <div id="root"></div>
    <div id="modal-root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

**Step 2: Modify `app/src/main.tsx`**

Replace the file with:

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import App from './App';
import { ModalProvider } from './components/Modal';
import './lib/fonts';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 5_000, refetchOnWindowFocus: false } },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <ModalProvider>
        <App />
      </ModalProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
```

**Step 3: Run the full app suite to verify no regressions**

```bash
pnpm --filter @snk/app test
```

Expected: all 173 baseline tests + new component tests still passing.

**Step 4: Run lint + typecheck**

```bash
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 5: Commit**

```bash
git add app/index.html app/src/main.tsx
git commit -m "feat(ui): wire ModalProvider in app root and add modal-root portal"
```

---

## Task 9: Refactor SettingsWindow to consume shared primitives

**Files:**
- Modify: `app/src/windows/settings/SettingsWindow.tsx`
- Verify: existing `app/src/windows/settings/SettingsWindow.test.tsx` still passes unchanged

**Context:** SettingsWindow currently has inline `SettingRow` (lines 16-34) and `Toggle` (lines 36-54). After this refactor, both come from `app/src/components/`. The five row-group sections (Capture, Clipboard, OCR, Startup, and the existing inline ClipboardSettings) gain `SettingsSection`. The Appearance section stays inline (it's grid-based, not row-group).

**Step 1: Run the existing SettingsWindow tests to confirm green baseline**

```bash
pnpm --filter @snk/app test -- --run src/windows/settings/SettingsWindow.test.tsx
```

Expected: 5 tests passing.

**Step 2: Replace the contents of `app/src/windows/settings/SettingsWindow.tsx`**

```tsx
import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  isEnabled as isAutostartEnabled,
  enable as enableAutostart,
  disable as disableAutostart,
} from '@tauri-apps/plugin-autostart';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';
import { THEMES, THEME_FAMILIES, familyOf, useTheme, type ThemeId } from '../../lib/theme';
import { ClipboardSettings } from './ClipboardSettings';
import { SettingRow } from '../../components/SettingRow';
import { SettingsSection } from '../../components/SettingsSection';
import { Toggle } from '../../components/Toggle';

function useSetting<T>(key: string, defaultValue: T): [T, (v: T) => void, boolean] {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: queryKeys.settings.one(key),
    queryFn: () => getSetting(key),
  });

  const value = data !== null && data !== undefined ? (data as T) : defaultValue;

  const update = (v: T) => {
    setSetting(key, v).then(() => {
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(key) });
    });
  };

  return [value, update, isLoading];
}

// Launch-at-login is managed by the autostart plugin, not the settings table,
// so it has its own little hook.
function useAutostart(): [boolean, (v: boolean) => void, boolean] {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ['autostart-enabled'],
    queryFn: () => isAutostartEnabled(),
  });

  const update = async (v: boolean) => {
    try {
      if (v) await enableAutostart();
      else await disableAutostart();
      queryClient.invalidateQueries({ queryKey: ['autostart-enabled'] });
    } catch (e) {
      console.error('autostart toggle failed', e);
    }
  };

  return [data ?? false, update, isLoading];
}

// DividerPreview is now colocated with each family's CSS — see
// themes/<family>.preview.tsx. SettingsWindow reads the matching component
// from THEME_FAMILIES[family].DividerPreview.
function ThemeCard({
  themeId,
  label,
  active,
  onSelect,
}: {
  themeId: ThemeId;
  label: string;
  active: boolean;
  onSelect: () => void;
}) {
  const family = familyOf(themeId);
  const isDark = themeId.endsWith('dark');
  const preview = THEME_FAMILIES[family].preview;
  const DividerPreview = THEME_FAMILIES[family].DividerPreview;

  const shapeClass = (() => {
    switch (preview.shape) {
      case 'round':
        return active ? 'rounded-2xl ring-2 ring-primary' : 'rounded-2xl border border-border';
      case 'memphis':
        return active ? 'memphis-card-accent' : 'memphis-card';
      case 'terminal':
        return active
          ? 'rounded-none ring-2 ring-amber-500 border border-amber-700'
          : 'rounded-none border border-amber-700';
      case 'card':
        return active
          ? 'rounded-none ring-4 ring-[#18181b] border-2 border-[#18181b]'
          : 'rounded-none border-2 border-[#18181b]';
    }
  })();

  const tagline = THEME_FAMILIES[family].tagline;
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const muted = isDark ? preview.mutedDark : preview.mutedLight;

  return (
    <button
      onClick={onSelect}
      className={`group relative text-left p-4 transition-transform hover:-translate-y-0.5 ${shapeClass}`}
      style={{ background: isDark ? preview.bgDark : preview.bgLight }}
    >
      <div className="flex gap-1.5 mb-3">
        {preview.swatches.map((c) => (
          <span
            key={c}
            className={`block ${preview.swatchShape === 'round' ? 'rounded-full' : ''}`}
            style={{
              width: 20,
              height: 20,
              background: c,
              border:
                preview.swatchShape === 'square'
                  ? `2px solid ${fg}`
                  : 'none',
            }}
          />
        ))}
      </div>

      <div
        className="text-sm leading-tight"
        style={{
          fontFamily: preview.displayFont,
          color: fg,
          letterSpacing: preview.shape === 'card' ? '0.08em' : undefined,
          textTransform: preview.shape === 'card' ? 'uppercase' : undefined,
        }}
      >
        {label}
      </div>

      <div
        className="text-[10px] mt-0.5 uppercase tracking-wider"
        style={{ fontFamily: preview.bodyFont, color: muted, opacity: 0.7 }}
      >
        {isDark ? 'dark' : 'light'}
      </div>

      <div
        className="text-[11px] mt-2 leading-snug italic"
        style={{ fontFamily: preview.bodyFont, color: muted }}
      >
        {tagline}
      </div>

      {/* Divider preview — colocated mockup at themes/<family>.preview.tsx.
          NOT the real .menu-divider because that's anchored to
          html[data-theme=X] and would inherit the active document theme
          inside the card. */}
      <div
        className="mt-3 px-2 py-1.5"
        style={{
          background: isDark ? preview.bgDark : preview.bgLight,
          filter: 'brightness(0.96)',
        }}
        aria-hidden
      >
        <DividerPreview mode={isDark ? 'dark' : 'light'} preview={preview} />
      </div>
    </button>
  );
}

export function SettingsWindow() {
  const { theme, setTheme } = useTheme();
  const [captureFormat, setCaptureFormat] = useSetting('capture.format', 'png');

  // Intercept window close so the X just hides; the webview stays alive
  // and the tray menu can re-open it instantly with show().
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('settings close listener failed', e));
    return () => cleanup?.();
  }, []);

  const [autoCopy, setAutoCopy] = useSetting('capture.auto_copy', true);
  const [jpgQuality, setJpgQuality] = useSetting('capture.jpg_quality', 90);
  const [historySize, setHistorySize] = useSetting('clipboard.history_size', 200);
  const [trackImages, setTrackImages] = useSetting('clipboard.track_images', true);
  const [trackFiles, setTrackFiles] = useSetting('clipboard.track_files', true);
  const [ocrEnabled, setOcrEnabled] = useSetting('ocr.enabled', true);
  const [autostart, setAutostart] = useAutostart();

  return (
    <main className="h-full flex flex-col bg-bg text-fg">
      <header className="px-5 py-4 border-b border-border flex items-baseline gap-3">
        <h1 className="font-display text-lg">Settings</h1>
        <span className="text-xs text-fg-muted">snapper-keeper</span>
      </header>
      <div className="flex-1 overflow-auto p-5 space-y-7">
        <section>
          <h2 className="font-display text-sm mb-3">Appearance</h2>
          <div className="grid grid-cols-2 gap-3">
            {THEMES.map((t) => (
              <ThemeCard
                key={t.id}
                themeId={t.id}
                label={t.label.split(' — ')[0]!}
                active={theme === t.id}
                onSelect={() => setTheme(t.id)}
              />
            ))}
          </div>
        </section>

        <SettingsSection title="Capture">
          <SettingRow label="Format">
            <select
              className="bg-surface-2 text-sm text-fg px-2 py-1 rounded border border-border"
              value={captureFormat as string}
              onChange={(e) => setCaptureFormat(e.target.value)}
            >
              <option value="png">PNG</option>
              <option value="jpg">JPG</option>
              <option value="webp">WebP</option>
            </select>
          </SettingRow>
          <SettingRow
            label="Auto-copy to clipboard"
            description="Copy capture to clipboard immediately after capture"
          >
            <Toggle value={autoCopy as boolean} onChange={setAutoCopy} />
          </SettingRow>
          {captureFormat === 'jpg' && (
            <SettingRow label="JPG quality" description="1–100">
              <input
                type="number"
                className="bg-surface-2 text-sm text-fg w-16 px-2 py-1 rounded border border-border"
                min={1}
                max={100}
                value={jpgQuality as number}
                onChange={(e) => setJpgQuality(Number(e.target.value))}
              />
            </SettingRow>
          )}
        </SettingsSection>

        <SettingsSection title="Clipboard">
          <SettingRow
            label="History size"
            description="Maximum number of clipboard items to keep"
          >
            <input
              type="number"
              className="bg-surface-2 text-sm text-fg w-20 px-2 py-1 rounded border border-border"
              min={10}
              max={1000}
              value={historySize as number}
              onChange={(e) => setHistorySize(Number(e.target.value))}
            />
          </SettingRow>
          <SettingRow
            label="Track images"
            description="Store copied images in clipboard history"
          >
            <Toggle value={trackImages as boolean} onChange={setTrackImages} />
          </SettingRow>
          <SettingRow
            label="Track files"
            description="Store copied file references in clipboard history"
          >
            <Toggle value={trackFiles as boolean} onChange={setTrackFiles} />
          </SettingRow>
        </SettingsSection>

        <section>
          <ClipboardSettings />
        </section>

        <SettingsSection title="OCR">
          <SettingRow
            label="Enable OCR"
            description="Automatically extract text from captures using Tesseract"
          >
            <Toggle value={ocrEnabled as boolean} onChange={setOcrEnabled} />
          </SettingRow>
        </SettingsSection>

        <SettingsSection title="Startup">
          <SettingRow
            label="Launch at login"
            description="Start snapper-keeper automatically when you sign in"
          >
            <Toggle value={autostart} onChange={setAutostart} />
          </SettingRow>
        </SettingsSection>
      </div>
    </main>
  );
}
```

**Step 3: Run the existing SettingsWindow tests to verify no regression**

```bash
pnpm --filter @snk/app test -- --run src/windows/settings/SettingsWindow.test.tsx
```

Expected: all 5 tests still passing. Note: one of the tests walks the DOM via `rowLabel.closest('div.flex')!` — this should keep working because `SettingRow` still emits `<div className="flex items-start...">`. If that test breaks, the row's wrapper structure changed; fix the row, not the test.

**Step 4: Run lint + typecheck**

```bash
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 5: Commit**

```bash
git add app/src/windows/settings/SettingsWindow.tsx
git commit -m "refactor(settings): consume shared SettingRow, Toggle, and SettingsSection"
```

---

## Task 10: Final verification — full suite + lint + typecheck across the workspace

**Files:** None modified. This is a verification-only task.

**Step 1: Run the full TS test suite**

```bash
pnpm -r --filter "@snk/*" --filter @snk/app test
```

Expected: 173 baseline + new component tests (Button 8 + Toggle 5 + SettingRow 4 + SettingsSection 3 + ModalProvider 4 + Modal 11 + useModal 4 = 39 new). Total ~212 tests, all passing.

**Step 2: Run lint across the app**

```bash
pnpm --filter @snk/app lint
```

Expected: clean.

**Step 3: Run typecheck across the app**

```bash
pnpm --filter @snk/app typecheck
```

Expected: clean.

**Step 4: Run the Vite build to confirm production bundling works**

```bash
pnpm --filter @snk/app build
```

Expected: build succeeds; no TS or bundling errors.

**Step 5: Report success and hand off**

After all four checks pass, the team-driven session is done. Push the branch and open a PR for review.

```bash
git push -u origin feat/react-shared-components
gh pr create --title "feat(ui): shared component infrastructure (Modal, SettingRow, etc.)" --body "..."
```

PR body should reference the design doc `docs/superpowers/specs/2026-05-25-react-cluster-design.md` and the issues this enables (#36, #62, #87, #78, #95).

---

## Self-review notes

1. **Spec coverage:** Modal (kind, focus trap, Esc/Enter, single-modal, portal, returns focus) — T5-T7. Button — T1. Toggle/SettingRow extraction — T2/T3. SettingsSection — T4. Provider wiring — T8. SettingsWindow refactor — T9. Verification — T10. All spec items covered.
2. **Placeholders:** None — every step has concrete code, exact paths, expected output.
3. **Naming consistency:** `useModal` returns `ModalAPI` with `confirm`/`alert`/`custom`. `ModalProvider` exposes `setModal`. `ModalState` discriminator is `kind`. Used consistently across T5-T7 + Modal.tsx + useModal.ts.
4. **Buildability:** Tasks are independent batches (T1-T5 parallel; T6 needs T5; T7 needs T6; T8 needs T7; T9 needs T2-T4; T10 last). Each task has runnable test commands with expected output.

## Plan-as-source-of-truth reminder

If any implementer finds a real bug in this plan (wrong API, broken test assertion, missing dependency), they report it to team-lead BEFORE coding around it. Lead approves the fix, edits the plan in place, then implementer codes against the corrected plan. Per memory `[[feedback_plan_as_source_of_truth]]`.
