import { useEffect, type ReactNode } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  isEnabled as isAutostartEnabled,
  enable as enableAutostart,
  disable as disableAutostart,
} from '@tauri-apps/plugin-autostart';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';
import { THEMES, THEME_FAMILIES, familyOf, useTheme, type ThemeId } from '../../lib/theme';

interface SettingRowProps {
  label: string;
  description?: string;
  children: ReactNode;
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-3">
      <div className="min-w-0">
        <div className="text-sm text-fg">{label}</div>
        {description && (
          <div className="text-[11px] text-fg-muted mt-0.5">{description}</div>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  // Pill: 44×22 (w-11 h-[22px] → wider so the thumb has clean travel)
  // Thumb: 16×16 (w-4 h-4)
  // Off: thumb sits at x=3 (3px gap). On: x=25 (3px gap on the right).
  return (
    <button
      type="button"
      className={`w-11 h-[22px] rounded-full relative transition-colors border border-border ${
        value ? 'bg-primary' : 'bg-surface-2'
      }`}
      onClick={() => onChange(!value)}
    >
      <span
        className="absolute top-[2px] w-4 h-4 rounded-full bg-bg transition-[left] duration-150"
        style={{ left: value ? 'calc(100% - 18px)' : '2px' }}
      />
    </button>
  );
}

function useSetting<T>(key: string, defaultValue: T): [T, (v: T) => void, boolean] {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: queryKeys.settings.one(key),
    queryFn: () => getSetting(key),
  });

  const value = data !== null && data !== undefined ? (data as T) : defaultValue;

  const update = (v: T) => {
    setSetting(key, v).then(() => {
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(key) });
    });
  };

  return [value, update, isLoading];
}

// Launch-at-login is managed by the autostart plugin, not the settings table,
// so it has its own little hook.
function useAutostart(): [boolean, (v: boolean) => void, boolean] {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ['autostart-enabled'],
    queryFn: () => isAutostartEnabled(),
  });

  const update = async (v: boolean) => {
    try {
      if (v) await enableAutostart();
      else await disableAutostart();
      queryClient.invalidateQueries({ queryKey: ['autostart-enabled'] });
    } catch (e) {
      console.error('autostart toggle failed', e);
    }
  };

  return [data ?? false, update, isLoading];
}

function ThemeCard({
  themeId,
  label,
  active,
  onSelect,
}: {
  themeId: ThemeId;
  label: string;
  active: boolean;
  onSelect: () => void;
}) {
  const family = familyOf(themeId);
  const isDark = themeId.endsWith('dark');
  const preview = THEME_FAMILIES[family].preview;

  const shapeClass = (() => {
    switch (preview.shape) {
      case 'round':
        return active ? 'rounded-2xl ring-2 ring-primary' : 'rounded-2xl border border-border';
      case 'memphis':
        return active ? 'memphis-card-accent' : 'memphis-card';
      case 'terminal':
        return active
          ? 'rounded-none ring-2 ring-amber-500 border border-amber-700'
          : 'rounded-none border border-amber-700';
      case 'card':
        return active
          ? 'rounded-none ring-4 ring-[#18181b] border-2 border-[#18181b]'
          : 'rounded-none border-2 border-[#18181b]';
    }
  })();

  return (
    <button
      onClick={onSelect}
      className={`group relative text-left p-3 transition-transform hover:-translate-y-0.5 ${shapeClass}`}
      style={{ background: isDark ? preview.bgDark : preview.bgLight }}
    >
      <div className="flex gap-1 mb-3">
        {preview.swatches.map((c) => (
          <span
            key={c}
            className={`block ${preview.swatchShape === 'round' ? 'rounded-full' : ''}`}
            style={{
              width: 18,
              height: 18,
              background: c,
              border:
                preview.swatchShape === 'square'
                  ? `2px solid ${isDark ? preview.fgDark : preview.fgLight}`
                  : 'none',
            }}
          />
        ))}
      </div>
      <div
        className="text-xs"
        style={{
          fontFamily: preview.displayFont,
          color: isDark ? preview.fgDark : preview.fgLight,
          letterSpacing: preview.shape === 'card' ? '0.08em' : undefined,
          textTransform: preview.shape === 'card' ? 'uppercase' : undefined,
        }}
      >
        {label}
      </div>
      <div
        className="text-[10px] mt-0.5"
        style={{
          fontFamily: preview.bodyFont,
          color: isDark ? preview.mutedDark : preview.mutedLight,
        }}
      >
        {isDark ? 'dark' : 'light'}
      </div>
    </button>
  );
}

export function SettingsWindow() {
  const { theme, setTheme } = useTheme();
  const [captureFormat, setCaptureFormat] = useSetting('capture.format', 'png');

  // Intercept window close so the X just hides; the webview stays alive
  // and the tray menu can re-open it instantly with show().
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('settings close listener failed', e));
    return () => cleanup?.();
  }, []);

  const [autoCopy, setAutoCopy] = useSetting('capture.auto_copy', true);
  const [jpgQuality, setJpgQuality] = useSetting('capture.jpg_quality', 90);
  const [historySize, setHistorySize] = useSetting('clipboard.history_size', 200);
  const [trackImages, setTrackImages] = useSetting('clipboard.track_images', true);
  const [trackFiles, setTrackFiles] = useSetting('clipboard.track_files', true);
  const [ocrEnabled, setOcrEnabled] = useSetting('ocr.enabled', true);
  const [autostart, setAutostart] = useAutostart();

  return (
    <main className="h-full flex flex-col bg-bg text-fg">
      <header className="px-5 py-4 border-b border-border flex items-baseline gap-3">
        <h1 className="font-display text-lg">Settings</h1>
        <span className="text-xs text-fg-muted">snapper-keeper</span>
      </header>
      <div className="flex-1 overflow-auto p-5 space-y-7">
        <section>
          <h2 className="font-display text-sm mb-3">Appearance</h2>
          <div className="grid grid-cols-2 gap-3">
            {THEMES.map((t) => (
              <ThemeCard
                key={t.id}
                themeId={t.id}
                label={t.label.split(' — ')[0]!}
                active={theme === t.id}
                onSelect={() => setTheme(t.id)}
              />
            ))}
          </div>
        </section>

        <section>
          <h2 className="font-display text-sm mb-3">Capture</h2>
          <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
            <SettingRow label="Format">
              <select
                className="bg-surface-2 text-sm text-fg px-2 py-1 rounded border border-border"
                value={captureFormat as string}
                onChange={(e) => setCaptureFormat(e.target.value)}
              >
                <option value="png">PNG</option>
                <option value="jpg">JPG</option>
                <option value="webp">WebP</option>
              </select>
            </SettingRow>
            <SettingRow
              label="Auto-copy to clipboard"
              description="Copy capture to clipboard immediately after capture"
            >
              <Toggle value={autoCopy as boolean} onChange={setAutoCopy} />
            </SettingRow>
            {captureFormat === 'jpg' && (
              <SettingRow label="JPG quality" description="1–100">
                <input
                  type="number"
                  className="bg-surface-2 text-sm text-fg w-16 px-2 py-1 rounded border border-border"
                  min={1}
                  max={100}
                  value={jpgQuality as number}
                  onChange={(e) => setJpgQuality(Number(e.target.value))}
                />
              </SettingRow>
            )}
          </div>
        </section>

        <section>
          <h2 className="font-display text-sm mb-3">Clipboard</h2>
          <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
            <SettingRow
              label="History size"
              description="Maximum number of clipboard items to keep"
            >
              <input
                type="number"
                className="bg-surface-2 text-sm text-fg w-20 px-2 py-1 rounded border border-border"
                min={10}
                max={1000}
                value={historySize as number}
                onChange={(e) => setHistorySize(Number(e.target.value))}
              />
            </SettingRow>
            <SettingRow
              label="Track images"
              description="Store copied images in clipboard history"
            >
              <Toggle value={trackImages as boolean} onChange={setTrackImages} />
            </SettingRow>
            <SettingRow
              label="Track files"
              description="Store copied file references in clipboard history"
            >
              <Toggle value={trackFiles as boolean} onChange={setTrackFiles} />
            </SettingRow>
          </div>
        </section>

        <section>
          <h2 className="font-display text-sm mb-3">OCR</h2>
          <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
            <SettingRow
              label="Enable OCR"
              description="Automatically extract text from captures using Tesseract"
            >
              <Toggle value={ocrEnabled as boolean} onChange={setOcrEnabled} />
            </SettingRow>
          </div>
        </section>

        <section>
          <h2 className="font-display text-sm mb-3">Startup</h2>
          <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
            <SettingRow
              label="Launch at login"
              description="Start snapper-keeper automatically when you sign in"
            >
              <Toggle value={autostart} onChange={setAutostart} />
            </SettingRow>
          </div>
        </section>
      </div>
    </main>
  );
}
