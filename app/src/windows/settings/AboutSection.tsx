import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getVersion } from '@tauri-apps/api/app';
import { appDataDir, appLogDir } from '@tauri-apps/api/path';
import { listen } from '@tauri-apps/api/event';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';

import {
  checkForUpdate,
  getUpdateStatus,
  lastCheckedAt,
  restart,
  type UpdateStatus,
} from '@snk/updater';

import { SettingsSection } from '../../components/SettingsSection';
import { SettingRow } from '../../components/SettingRow';
import { Button } from '../../components/Button';
import { useModal } from '../../components/Modal';
import { isStoreEdition } from '../../lib/storeEdition';
import { useSetting } from './useSetting';

const PRIVACY_URL =
  'https://github.com/ehartye/snapper-keeper/blob/main/PRIVACY.md';
const LICENSE_URL =
  'https://github.com/ehartye/snapper-keeper/blob/main/LICENSE.md';

function formatStatus(s: UpdateStatus): string {
  switch (s.kind) {
    case 'idle':
      return 'Up to date';
    case 'checking':
      return 'Checking…';
    case 'available':
      return `Update available: v${s.version} (${s.urgency})`;
    case 'downloading':
      return `Downloading ${Math.round(s.progress)}%`;
    case 'ready':
      return `Ready to install v${s.version}`;
    case 'installing':
      return 'Installing update…';
    case 'error':
      return `Error (${s.reason})${s.retryable ? ', retry scheduled' : ''}`;
    case 'rejected-by-signature':
      return 'Update rejected: signature verification failed';
    case 'suppressed-by-policy':
      switch (s.reason) {
        case 'user-disabled':
          return 'Update checks disabled in Settings';
        case 'store-edition':
          return 'Updates are managed by Microsoft Store';
        case 'downgrade-blocked':
          return 'Update blocked: it would downgrade below the newest version seen (enable rollback in Settings to override)';
        default:
          return `Update checks suppressed (${s.reason})`;
      }
    case 'skipped':
      return `Skipped until ${new Date(s.until_epoch_ms).toLocaleString()}`;
    default: {
      // Exhaustiveness guard — also satisfies TS's "function lacks
      // ending return statement" check. If a new variant is added,
      // this line becomes a compile error.
      const _exhaustive: never = s;
      return _exhaustive;
    }
  }
}

function formatRelative(ts: number | null): string {
  if (ts === null) return 'never';
  const diffMs = Date.now() - ts;
  if (diffMs < 60_000) return 'just now';
  if (diffMs < 3_600_000) return `${Math.floor(diffMs / 60_000)}m ago`;
  if (diffMs < 86_400_000) return `${Math.floor(diffMs / 3_600_000)}h ago`;
  return `${Math.floor(diffMs / 86_400_000)}d ago`;
}

export function AboutSection() {
  const modal = useModal();
  const storeEdition = isStoreEdition();
  const [updaterEnabled] = useSetting('updater.enabled', true);

  const versionQ = useQuery({
    queryKey: ['app-version'],
    queryFn: () => getVersion(),
  });
  const dataDirQ = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => appDataDir(),
  });
  const logDirQ = useQuery({
    queryKey: ['app-log-dir'],
    queryFn: () => appLogDir(),
  });

  const [status, setStatus] = useState<UpdateStatus>({ kind: 'idle' });
  const [lastCheck, setLastCheck] = useState<number | null>(null);
  const [restartPrompted, setRestartPrompted] = useState(false);

  useEffect(() => {
    if (storeEdition) return;

    let cancelled = false;
    void getUpdateStatus().then((s) => {
      if (!cancelled) setStatus(s);
    });
    void lastCheckedAt().then((ts) => {
      if (!cancelled) setLastCheck(ts);
    });
    const unlistenPromise = listen<UpdateStatus>(
      'updater:status-changed',
      (e) => {
        setStatus(e.payload);
        if (
          e.payload.kind === 'checking' ||
          e.payload.kind === 'idle' ||
          e.payload.kind === 'error'
        ) {
          void lastCheckedAt().then((ts) => setLastCheck(ts));
        }
      },
    );
    return () => {
      cancelled = true;
      void unlistenPromise.then((fn) => fn());
    };
  }, [storeEdition]);

  const effectiveStatus = useMemo<UpdateStatus>(() => {
    if (storeEdition) {
      return { kind: 'suppressed-by-policy', reason: 'store-edition' };
    }

    if (!updaterEnabled) {
      return { kind: 'suppressed-by-policy', reason: 'user-disabled' };
    }

    return status;
  }, [status, storeEdition, updaterEnabled]);

  useEffect(() => {
    if (effectiveStatus.kind === 'ready' && !restartPrompted) {
      setRestartPrompted(true);
      modal.confirm({
        title: 'Update ready',
        body: `Update v${effectiveStatus.version} is ready. Restart now to install?`,
        confirmLabel: 'Restart',
        cancelLabel: 'Later',
        onConfirm: () => restart(),
      });
    }
  }, [effectiveStatus, restartPrompted, modal]);

  const isChecking =
    effectiveStatus.kind === 'checking' ||
    effectiveStatus.kind === 'downloading' ||
    effectiveStatus.kind === 'installing';
  const sha = __GIT_SHA__;
  const fingerprint = storeEdition ? 'not bundled in store edition' : __UPDATER_FINGERPRINT__;

  return (
    <SettingsSection title="About">
      <SettingRow label="Version">
        <span className="text-sm text-fg-muted font-mono">
          {versionQ.data ? `${versionQ.data} (${sha})` : `… (${sha})`}
        </span>
      </SettingRow>
      <SettingRow label="Data directory" description={dataDirQ.data ?? ''}>
        <Button
          variant="secondary"
          onClick={() => dataDirQ.data && void openPath(dataDirQ.data)}
          disabled={!dataDirQ.data}
        >
          Open
        </Button>
      </SettingRow>
      <SettingRow label="Log directory" description={logDirQ.data ?? ''}>
        <Button
          variant="secondary"
          onClick={() => logDirQ.data && void openPath(logDirQ.data)}
          disabled={!logDirQ.data}
        >
          Open
        </Button>
      </SettingRow>
      <SettingRow
        label="Fingerprint"
        description="Updater public key identifier (verify against release notes)"
      >
        <span className="text-xs text-fg-muted font-mono">{fingerprint}</span>
      </SettingRow>
      <SettingRow label="Last check">
        <span
          className="text-sm text-fg-muted"
          title={lastCheck ? new Date(lastCheck).toISOString() : ''}
        >
          {formatRelative(lastCheck)}
        </span>
      </SettingRow>
      <SettingRow label="Status" description={formatStatus(effectiveStatus)}>
        <Button
          onClick={() => void checkForUpdate()}
          disabled={isChecking || storeEdition || !updaterEnabled}
        >
          {storeEdition ? 'Managed by Store' : 'Check Now'}
        </Button>
      </SettingRow>
      <SettingRow label="Privacy">
        <Button variant="secondary" onClick={() => void openUrl(PRIVACY_URL)}>
          Privacy
        </Button>
      </SettingRow>
      <SettingRow label="License">
        <Button variant="secondary" onClick={() => void openUrl(LICENSE_URL)}>
          License
        </Button>
      </SettingRow>
    </SettingsSection>
  );
}
