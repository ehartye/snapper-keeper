import type { ReactNode } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

interface SettingRowProps {
  label: string;
  description?: string;
  children: ReactNode;
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-2">
      <div>
        <div className="text-sm text-slate-200">{label}</div>
        {description && <div className="text-[10px] text-slate-500">{description}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      className={`w-9 h-5 rounded-full relative transition-colors ${value ? 'bg-blue-600' : 'bg-slate-600'}`}
      onClick={() => onChange(!value)}
    >
      <span
        className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${value ? 'translate-x-4' : 'translate-x-0.5'}`}
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

export function SettingsWindow() {
  const [captureFormat, setCaptureFormat] = useSetting('capture.format', 'png');
  const [autoCopy, setAutoCopy] = useSetting('capture.auto_copy', true);
  const [jpgQuality, setJpgQuality] = useSetting('capture.jpg_quality', 90);
  const [historySize, setHistorySize] = useSetting('clipboard.history_size', 200);
  const [trackImages, setTrackImages] = useSetting('clipboard.track_images', true);
  const [trackFiles, setTrackFiles] = useSetting('clipboard.track_files', true);
  const [ocrEnabled, setOcrEnabled] = useSetting('ocr.enabled', true);

  return (
    <main className="h-full flex flex-col bg-slate-950 text-slate-100">
      <header className="px-4 py-3 border-b border-slate-800">
        <h1 className="text-sm font-semibold">Settings</h1>
      </header>
      <div className="flex-1 overflow-auto p-4 space-y-6">
        <section>
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Capture</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="Format">
              <select
                className="bg-slate-800 text-sm text-slate-200 px-2 py-1 rounded border border-slate-700"
                value={captureFormat as string}
                onChange={(e) => setCaptureFormat(e.target.value)}
              >
                <option value="png">PNG</option>
                <option value="jpg">JPG</option>
                <option value="webp">WebP</option>
              </select>
            </SettingRow>
            <SettingRow label="Auto-copy to clipboard" description="Copy capture to clipboard immediately after capture">
              <Toggle value={autoCopy as boolean} onChange={setAutoCopy} />
            </SettingRow>
            {captureFormat === 'jpg' && (
              <SettingRow label="JPG quality" description="1–100">
                <input
                  type="number"
                  className="bg-slate-800 text-sm text-slate-200 w-16 px-2 py-1 rounded border border-slate-700"
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
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Clipboard</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="History size" description="Maximum number of clipboard items to keep">
              <input
                type="number"
                className="bg-slate-800 text-sm text-slate-200 w-20 px-2 py-1 rounded border border-slate-700"
                min={10}
                max={1000}
                value={historySize as number}
                onChange={(e) => setHistorySize(Number(e.target.value))}
              />
            </SettingRow>
            <SettingRow label="Track images" description="Store copied images in clipboard history">
              <Toggle value={trackImages as boolean} onChange={setTrackImages} />
            </SettingRow>
            <SettingRow label="Track files" description="Store copied file references in clipboard history">
              <Toggle value={trackFiles as boolean} onChange={setTrackFiles} />
            </SettingRow>
          </div>
        </section>

        <section>
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">OCR</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="Enable OCR" description="Automatically extract text from captures using Tesseract">
              <Toggle value={ocrEnabled as boolean} onChange={setOcrEnabled} />
            </SettingRow>
          </div>
        </section>
      </div>
    </main>
  );
}
