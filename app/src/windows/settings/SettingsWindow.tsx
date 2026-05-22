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
import {
  THEMES,
  THEME_FAMILIES,
  familyOf,
  useTheme,
  type FamilyPreview,
  type ThemeFamily,
  type ThemeId,
} from '../../lib/theme';

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

/**
 * Inline approximation of each family's .menu-divider for the card preview.
 * Rendered with inline styles rather than the real CSS so that one card's
 * theme can't be hijacked by the active document theme's cascade (the actual
 * .menu-divider rules in app/src/themes/*.css are anchored to html so they
 * never bleed into card scope).
 */
function DividerPreview({
  family,
  mode,
  preview,
}: {
  family: ThemeFamily;
  mode: 'light' | 'dark';
  preview: FamilyPreview;
}) {
  const isDark = mode === 'dark';
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const muted = isDark ? preview.mutedDark : preview.mutedLight;
  const bg = isDark ? preview.bgDark : preview.bgLight;
  // Swatches are conventionally [primary, accent, third, fourth] per family.
  // Non-null asserts are safe — every family registers all four (enforced by
  // the FamilyPreview shape).
  const primary = preview.swatches[0]!;
  const accent = preview.swatches[1]!;
  const third = preview.swatches[2]!;

  switch (family) {
    case 'holo':
      return (
        <div
          style={{
            height: 12,
            backgroundImage: `linear-gradient(to right, transparent 0%, ${third} 25%, ${primary} 50%, ${accent} 75%, transparent 100%)`,
            backgroundSize: '100% 2px',
            backgroundRepeat: 'no-repeat',
            backgroundPosition: 'center',
          }}
        />
      );
    case 'memphis': {
      const stroke = encodeURIComponent(fg);
      return (
        <div
          style={{
            height: 12,
            backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 12' preserveAspectRatio='none'><path d='M0 6 Q 12.5 0 25 6 T 50 6 T 75 6 T 100 6' stroke='${stroke}' stroke-width='2.5' fill='none' /></svg>")`,
            backgroundRepeat: 'repeat-x',
            backgroundSize: '50px 12px',
          }}
        />
      );
    }
    case 'robotic':
      return (
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            gap: 6,
            height: 12,
            fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
            fontSize: 8,
            letterSpacing: '0.5px',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
          }}
        >
          <span style={{ color: accent }}>0x00FF</span>
          <span style={{ color: muted }}>41 6C 6C 0A 2D 54 41 47 53</span>
        </div>
      );
    case 'corporate':
      return (
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: 12,
            backgroundImage: `linear-gradient(to right, ${fg}, ${fg})`,
            backgroundSize: '100% 1px',
            backgroundPosition: 'center',
            backgroundRepeat: 'no-repeat',
          }}
        >
          <span
            style={{
              background: bg,
              padding: '0 6px',
              fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
              fontSize: 8,
              fontWeight: 600,
              letterSpacing: '1px',
              color: muted,
              textTransform: 'uppercase',
            }}
          >
            № I
          </span>
        </div>
      );
    case 'wabi-sabi': {
      const brushColor = encodeURIComponent(isDark ? '#c8a878' : '#2a1f1a');
      return (
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: 14,
            backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 4' preserveAspectRatio='none'><path d='M2 2.2 Q 22 0.8 50 2.5 Q 78 4.2 98 2' stroke='${brushColor}' stroke-width='0.6' fill='none' stroke-linecap='round' opacity='0.55' /></svg>")`,
            backgroundSize: '100% 4px',
            backgroundPosition: 'center',
            backgroundRepeat: 'no-repeat',
          }}
        >
          <span
            style={{
              background: primary,
              color: bg,
              fontFamily:
                "'Shippori Mincho', 'Yu Mincho', 'Hiragino Mincho ProN', 'Noto Serif JP', serif",
              fontWeight: 800,
              fontSize: 10,
              lineHeight: 1,
              padding: '2px 3px',
              borderRadius: 1,
              transform: 'rotate(-3deg)',
              boxShadow: 'inset 0 0 2px rgba(0,0,0,0.4)',
            }}
          >
            章
          </span>
        </div>
      );
    }
    case 'riso':
      return (
        <div style={{ position: 'relative', height: 12 }}>
          <div
            style={{
              position: 'absolute',
              left: 0,
              right: 0,
              top: 5,
              height: 1.5,
              background: primary,
              opacity: 0.85,
            }}
          />
          <div
            style={{
              position: 'absolute',
              left: 2,
              right: 0,
              top: 7,
              height: 1.5,
              background: accent,
              opacity: 0.85,
            }}
          />
        </div>
      );
    case 'constructivist':
      return (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, height: 14 }}>
          <span
            style={{
              flex: '0 0 10px',
              width: 10,
              height: 10,
              background: primary,
              transform: 'rotate(15deg)',
            }}
          />
          <span style={{ flex: 1, height: 2, background: fg }} />
        </div>
      );
    case 'atomic': {
      const starFill = encodeURIComponent(primary);
      return (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: 14,
            backgroundImage: `linear-gradient(to right, ${fg}, ${fg})`,
            backgroundSize: '100% 1px',
            backgroundPosition: 'center',
            backgroundRepeat: 'no-repeat',
            opacity: 1,
          }}
        >
          <span
            style={{
              width: 12,
              height: 12,
              backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'><path d='M6 0.5 L7 5 L11.5 6 L7 7 L6 11.5 L5 7 L0.5 6 L5 5 Z' fill='${starFill}' /></svg>")`,
              backgroundSize: '12px 12px',
              backgroundColor: bg,
              backgroundRepeat: 'no-repeat',
              backgroundPosition: 'center',
              padding: '0 4px',
              boxSizing: 'content-box',
            }}
          />
        </div>
      );
    }
  }
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

  const tagline = THEME_FAMILIES[family].tagline;
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const muted = isDark ? preview.mutedDark : preview.mutedLight;

  return (
    <button
      onClick={onSelect}
      className={`group relative text-left p-4 transition-transform hover:-translate-y-0.5 ${shapeClass}`}
      style={{ background: isDark ? preview.bgDark : preview.bgLight }}
    >
      <div className="flex gap-1.5 mb-3">
        {preview.swatches.map((c) => (
          <span
            key={c}
            className={`block ${preview.swatchShape === 'round' ? 'rounded-full' : ''}`}
            style={{
              width: 20,
              height: 20,
              background: c,
              border:
                preview.swatchShape === 'square'
                  ? `2px solid ${fg}`
                  : 'none',
            }}
          />
        ))}
      </div>

      <div
        className="text-sm leading-tight"
        style={{
          fontFamily: preview.displayFont,
          color: fg,
          letterSpacing: preview.shape === 'card' ? '0.08em' : undefined,
          textTransform: preview.shape === 'card' ? 'uppercase' : undefined,
        }}
      >
        {label}
      </div>

      <div
        className="text-[10px] mt-0.5 uppercase tracking-wider"
        style={{ fontFamily: preview.bodyFont, color: muted, opacity: 0.7 }}
      >
        {isDark ? 'dark' : 'light'}
      </div>

      <div
        className="text-[11px] mt-2 leading-snug italic"
        style={{ fontFamily: preview.bodyFont, color: muted }}
      >
        {tagline}
      </div>

      {/* Divider preview — inline mockup per family. NOT the real .menu-divider
          because that's anchored to html[data-theme=X] and would inherit the
          active document theme inside the card. See DividerPreview above. */}
      <div
        className="mt-3 px-2 py-1.5"
        style={{
          background: isDark ? preview.bgDark : preview.bgLight,
          filter: 'brightness(0.96)',
        }}
        aria-hidden
      >
        <DividerPreview family={family} mode={isDark ? 'dark' : 'light'} preview={preview} />
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
