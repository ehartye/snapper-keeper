import { SettingRow } from '../../components/SettingRow';
import { SettingsSection } from '../../components/SettingsSection';
import { Toggle } from '../../components/Toggle';
import { isStoreEdition } from '../../lib/storeEdition';
import { useSetting } from './useSetting';

const UPDATER_ENABLED_KEY = 'updater.enabled';
const UPDATER_ALLOW_ROLLBACK_KEY = 'updater.allow_rollback';

export function UpdateSettings() {
  const storeEdition = isStoreEdition();
  const [updaterEnabled, setUpdaterEnabled] = useSetting(UPDATER_ENABLED_KEY, true);
  const [allowRollback, setAllowRollback] = useSetting(UPDATER_ALLOW_ROLLBACK_KEY, false);

  return (
    <SettingsSection title="Updates">
      {storeEdition ? (
        <SettingRow
          label="Update checks"
          description="Microsoft Store builds do not include the in-app updater."
        >
          <span className="text-sm text-fg-muted">Managed by Microsoft Store</span>
        </SettingRow>
      ) : (
        <>
          <SettingRow
            label="Enable update checks"
            description="Allow snapper-keeper to contact GitHub Releases on launch and once every 24 hours."
          >
            <Toggle value={updaterEnabled} onChange={setUpdaterEnabled} />
          </SettingRow>
          <SettingRow
            label="Allow rollback to older versions"
            description="Off by default. When off, the updater refuses any release older than the newest one it has already seen, blocking downgrade attacks. Only enable this if you intentionally need to install an older version."
          >
            <Toggle value={allowRollback} onChange={setAllowRollback} />
          </SettingRow>
        </>
      )}
    </SettingsSection>
  );
}
