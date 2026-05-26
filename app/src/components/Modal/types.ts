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
