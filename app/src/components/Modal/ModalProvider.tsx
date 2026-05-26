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
