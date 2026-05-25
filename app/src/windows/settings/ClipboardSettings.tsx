import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { getSetting, setSetting } from '@snk/library';
import {
  APP_BLOCKLIST_SETTING_KEY,
  detectFrontmostApp,
  type BlocklistEntry,
  type SourceApp,
} from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';

function readEntries(value: unknown): BlocklistEntry[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (e: unknown): e is BlocklistEntry =>
      typeof e === 'object' &&
      e !== null &&
      typeof (e as { identifier?: unknown }).identifier === 'string' &&
      typeof (e as { display_name?: unknown }).display_name === 'string' &&
      ((e as { kind?: unknown }).kind === 'macos_bundle_id' ||
        (e as { kind?: unknown }).kind === 'windows_exe'),
  );
}

export function ClipboardSettings() {
  const queryClient = useQueryClient();
  const { data: rawValue } = useQuery({
    queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    queryFn: () => getSetting(APP_BLOCKLIST_SETTING_KEY),
  });
  const entries = readEntries(rawValue);

  const [addOpen, setAddOpen] = useState(false);
  const [confirmFrontmost, setConfirmFrontmost] = useState<SourceApp | null>(null);

  async function persist(next: BlocklistEntry[]) {
    await setSetting(APP_BLOCKLIST_SETTING_KEY, next);
    await queryClient.invalidateQueries({
      queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    });
  }

  function remove(identifier: string, kind: BlocklistEntry['kind']) {
    void persist(
      entries.filter((e) => !(e.identifier === identifier && e.kind === kind)),
    );
  }

  async function addFromFrontmost() {
    const app = await detectFrontmostApp();
    if (app) setConfirmFrontmost(app);
  }

  return (
    <div>
      <h2 className="text-sm font-display uppercase tracking-wider text-fg-muted mb-2">
        Excluded apps
      </h2>
      <p className="text-[11px] text-fg-muted mb-3">
        Clipboard events from these apps are never recorded. OS-level
        &quot;concealed&quot; flags are always honored regardless of this list.
      </p>

      <ul className="border border-border rounded">
        {entries.length === 0 && (
          <li className="px-3 py-2 text-xs text-fg-muted">
            No exclusions configured.
          </li>
        )}
        {entries.map((e) => (
          <li
            key={`${e.kind}:${e.identifier}`}
            className="flex items-center justify-between px-3 py-2 border-b border-border last:border-0"
          >
            <div>
              <div className="text-sm text-fg">{e.display_name}</div>
              <div className="text-[10px] text-fg-muted">
                {e.identifier} · {e.kind}
              </div>
            </div>
            <button
              onClick={() => remove(e.identifier, e.kind)}
              className="text-fg-muted hover:text-danger text-xs"
              aria-label={`Remove ${e.display_name}`}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      <div className="flex gap-2 mt-3">
        <button
          onClick={() => setAddOpen(true)}
          className="text-xs text-fg hover:text-primary"
        >
          + Add app…
        </button>
        <button
          onClick={addFromFrontmost}
          className="text-xs text-fg hover:text-primary"
        >
          + Add from frontmost app
        </button>
      </div>

      {addOpen && (
        <AddAppModal
          existing={entries}
          onClose={() => setAddOpen(false)}
          onAdd={(entry) => {
            void persist([...entries, entry]);
            setAddOpen(false);
          }}
        />
      )}
      {confirmFrontmost && (
        <ConfirmFrontmostModal
          app={confirmFrontmost}
          existing={entries}
          onClose={() => setConfirmFrontmost(null)}
          onConfirm={(entry) => {
            void persist([...entries, entry]);
            setConfirmFrontmost(null);
          }}
        />
      )}
    </div>
  );
}

interface AddAppModalProps {
  existing: BlocklistEntry[];
  onClose: () => void;
  onAdd: (entry: BlocklistEntry) => void;
}

function AddAppModal({ existing, onClose, onAdd }: AddAppModalProps) {
  const [identifier, setIdentifier] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [kind, setKind] = useState<BlocklistEntry['kind']>('macos_bundle_id');
  const [error, setError] = useState<string | null>(null);

  function submit() {
    const id = identifier.trim();
    if (!id) {
      setError('Identifier is required.');
      return;
    }
    const dup = existing.find((e) => e.identifier === id && e.kind === kind);
    if (dup) {
      setError('Already in the list.');
      return;
    }
    onAdd({
      identifier: id,
      display_name: displayName.trim() || id,
      kind,
    });
  }

  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-bg border-2 border-border rounded p-4 w-80"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-display uppercase mb-3">Add excluded app</h3>
        <label className="block text-[10px] text-fg-muted mb-1">Kind</label>
        <select
          value={kind}
          onChange={(e) => setKind(e.target.value as BlocklistEntry['kind'])}
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
        >
          <option value="macos_bundle_id">macOS bundle ID</option>
          <option value="windows_exe">Windows exe</option>
        </select>
        <label className="block text-[10px] text-fg-muted mb-1">Identifier</label>
        <input
          value={identifier}
          onChange={(e) => setIdentifier(e.target.value)}
          placeholder={
            kind === 'macos_bundle_id'
              ? 'com.example.app'
              : 'example.exe'
          }
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
        />
        <label className="block text-[10px] text-fg-muted mb-1">
          Display name (optional)
        </label>
        <input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-3"
        />
        {error && <div className="text-[10px] text-danger mb-2">{error}</div>}
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="text-xs text-fg-muted">
            Cancel
          </button>
          <button
            onClick={submit}
            className="text-xs text-bg bg-primary px-2 py-1 rounded"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}

interface ConfirmFrontmostModalProps {
  app: SourceApp;
  existing: BlocklistEntry[];
  onClose: () => void;
  onConfirm: (entry: BlocklistEntry) => void;
}

function ConfirmFrontmostModal({
  app,
  existing,
  onClose,
  onConfirm,
}: ConfirmFrontmostModalProps) {
  const dup = existing.find(
    (e) => e.identifier === app.identifier && e.kind === app.kind,
  );
  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-bg border-2 border-border rounded p-4 w-80"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-display uppercase mb-3">
          Block frontmost app?
        </h3>
        <div className="text-sm text-fg mb-1">{app.display_name}</div>
        <div className="text-[10px] text-fg-muted mb-3">
          {app.identifier} · {app.kind}
        </div>
        {dup && (
          <div className="text-[10px] text-danger mb-2">
            This app is already in the list.
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="text-xs text-fg-muted">
            Cancel
          </button>
          <button
            disabled={!!dup}
            onClick={() =>
              onConfirm({
                identifier: app.identifier,
                display_name: app.display_name,
                kind: app.kind,
              })
            }
            className="text-xs text-bg bg-primary px-2 py-1 rounded disabled:opacity-50"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
