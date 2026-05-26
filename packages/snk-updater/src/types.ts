export type PolicySuppressionReason = 'user-disabled' | 'store-edition';

export type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'available'; version: string }
  | { kind: 'downloading'; percent: number }
  | { kind: 'ready'; version: string }
  | { kind: 'suppressed-by-policy'; reason: PolicySuppressionReason }
  | { kind: 'error'; detail: string };
