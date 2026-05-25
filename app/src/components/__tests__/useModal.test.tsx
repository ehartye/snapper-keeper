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
