import { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { Button } from '../Button';
import { useModalContext } from './ModalProvider';

export function Modal() {
  const { modal, setModal } = useModalContext();
  const previouslyFocused = useRef<HTMLElement | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  const close = () => setModal(null);

  useEffect(() => {
    if (modal) {
      previouslyFocused.current = document.activeElement as HTMLElement | null;
      requestAnimationFrame(() => {
        dialogRef.current?.querySelector<HTMLElement>('button, input, textarea, [tabindex]')?.focus();
      });
    } else if (previouslyFocused.current) {
      previouslyFocused.current.focus();
      previouslyFocused.current = null;
    }
  }, [modal]);

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
