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
