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
