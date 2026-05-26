export type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'available'; version: string; urgency: Urgency }
  | { kind: 'downloading'; progress: number }
  | { kind: 'ready'; version: string }
  | { kind: 'installing' }
  | { kind: 'error'; reason: UpdateErrorKind; retryable: boolean }
  | { kind: 'rejected-by-signature' }
  | { kind: 'suppressed-by-policy'; reason: SuppressionReason }
  | { kind: 'skipped'; until_epoch_ms: number };

export type Urgency = 'low' | 'normal' | 'high' | 'critical';

export type UpdateErrorKind =
  | 'check-failed'
  | 'download-failed'
  | 'install-failed'
  | 'signature-verification-failed'
  | 'unknown';

export type SuppressionReason =
  | 'user-disabled'
  | 'store-edition'
  | 'downgrade-blocked'
  | 'unknown-policy';
