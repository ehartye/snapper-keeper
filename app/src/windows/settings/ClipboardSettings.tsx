import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';

import { getSetting, setSetting } from '@snk/library';
import {
  APP_BLOCKLIST_SETTING_KEY,
  detectFrontmostApp,
  type BlocklistEntry,
  type SourceApp,
} from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';
import { useModal } from '../../components/Modal';
import { Button } from '../../components/Button';

export function readEntries(value: unknown): BlocklistEntry[] {
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
  const modal = useModal();
  const { data: rawValue } = useQuery({
    queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    queryFn: () => getSetting(APP_BLOCKLIST_SETTING_KEY),
  });
  const entries = readEntries(rawValue);

  // Mirror entries in a ref so the modal's render closure (which captures
  // values at modal.custom() invocation time, not on every parent render)
  // can read fresh entries for the dup check at submit time.
  const entriesRef = useRef<BlocklistEntry[]>(entries);
  entriesRef.current = entries;

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

  function openAddApp() {
    modal.custom({
      title: 'Add excluded app',
      render: ({ close }) => (
        <AddAppForm
          getExisting={() => entriesRef.current}
          onAdd={(entry) => {
            void persist([...entriesRef.current, entry]);
            close();
          }}
          onCancel={close}
        />
      ),
    });
  }

  async function addFromFrontmost() {
    const app = await detectFrontmostApp();
    if (!app) return;
    modal.custom({
      title: 'Block frontmost app?',
      render: ({ close }) => (
        <ConfirmFrontmostBody
          app={app}
          getExisting={() => entriesRef.current}
          onConfirm={(entry) => {
            void persist([...entriesRef.current, entry]);
            close();
          }}
          onCancel={close}
        />
      ),
    });
  }

  return (
    <div>
      <h2 className="font-display text-sm mb-3">Excluded apps</h2>
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
          onClick={openAddApp}
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
    </div>
  );
}

interface AddAppFormProps {
  getExisting: () => BlocklistEntry[];
  onAdd: (entry: BlocklistEntry) => void;
  onCancel: () => void;
}

function AddAppForm({ getExisting, onAdd, onCancel }: AddAppFormProps) {
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
    const dup = getExisting().find(
      (e) => e.identifier === id && e.kind === kind,
    );
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
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
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
        placeholder={kind === 'macos_bundle_id' ? 'com.example.app' : 'example.exe'}
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
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">Add</Button>
      </div>
    </form>
  );
}

interface ConfirmFrontmostBodyProps {
  app: SourceApp;
  getExisting: () => BlocklistEntry[];
  onConfirm: (entry: BlocklistEntry) => void;
  onCancel: () => void;
}

function ConfirmFrontmostBody({
  app,
  getExisting,
  onConfirm,
  onCancel,
}: ConfirmFrontmostBodyProps) {
  const dup = getExisting().find(
    (e) => e.identifier === app.identifier && e.kind === app.kind,
  );
  return (
    <div>
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
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          disabled={!!dup}
          onClick={() =>
            onConfirm({
              identifier: app.identifier,
              display_name: app.display_name,
              kind: app.kind,
            })
          }
        >
          Add
        </Button>
      </div>
    </div>
  );
}
