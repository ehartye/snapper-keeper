import { SettingRow } from '../../components/SettingRow';
import { SettingsSection } from '../../components/SettingsSection';
import { Toggle } from '../../components/Toggle';
import { isStoreEdition } from '../../lib/storeEdition';
import { useSetting } from './useSetting';

const UPDATER_ENABLED_KEY = 'updater.enabled';

export function UpdateSettings() {
  const storeEdition = isStoreEdition();
  const [updaterEnabled, setUpdaterEnabled] = useSetting(UPDATER_ENABLED_KEY, true);

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
        <SettingRow
          label="Enable update checks"
          description="Allow snapper-keeper to contact GitHub Releases on launch and once every 24 hours."
        >
          <Toggle value={updaterEnabled} onChange={setUpdaterEnabled} />
        </SettingRow>
      )}
    </SettingsSection>
  );
}
