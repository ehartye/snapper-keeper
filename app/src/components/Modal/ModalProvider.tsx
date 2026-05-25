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
