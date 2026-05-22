import { useState, type ReactNode } from 'react';

import { setSetting } from '@snk/library';

type Step = 'welcome' | 'hotkeys' | 'library' | 'done';

const DEFAULT_HOTKEYS = [
  { action: 'Capture region', chord: 'Ctrl+Shift+4' },
  { action: 'Capture window', chord: 'Ctrl+Shift+5' },
  { action: 'Capture screen', chord: 'Ctrl+Shift+3' },
  { action: 'Timed capture', chord: 'Ctrl+Shift+6' },
  { action: 'Clipboard history', chord: 'Ctrl+Shift+V' },
  { action: 'Open library', chord: 'Ctrl+Shift+L' },
];

interface Props {
  onComplete: () => void;
}

function PrimaryButton({ onClick, children }: { onClick: () => void; children: ReactNode }) {
  return (
    <button
      className="font-display text-xs uppercase tracking-widest px-6 py-2.5 bg-primary text-bg border-2 border-border shadow-[3px_3px_0_0_var(--border)] hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[1px_1px_0_0_var(--border)] active:translate-x-1 active:translate-y-1 active:shadow-none transition-transform"
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export function FirstRunWizard({ onComplete }: Props) {
  const [step, setStep] = useState<Step>('welcome');

  const finish = async () => {
    await setSetting('firstrun.completed', true);
    onComplete();
  };

  return (
    <div className="fixed inset-0 bg-bg flex items-center justify-center z-50">
      <div className="max-w-md w-full mx-4 bg-surface border-2 border-border rounded-2xl p-8 shadow-[8px_8px_0_0_var(--accent-2)]">
        {step === 'welcome' && (
          <div className="text-center space-y-5">
            <h1 className="font-display text-3xl">
              <span className="holo-shimmer bg-clip-text text-transparent">welcome to</span>
              <br />
              <span>snapper-keeper</span>
            </h1>
            <p className="text-sm text-fg-muted">
              Screen capture with OCR search, plus clipboard history with instant paste.
              Let&apos;s get you set up.
            </p>
            <div className="pt-2">
              <PrimaryButton onClick={() => setStep('hotkeys')}>get started</PrimaryButton>
            </div>
          </div>
        )}

        {step === 'hotkeys' && (
          <div className="space-y-5">
            <h2 className="font-display text-xl">⌨ keyboard shortcuts</h2>
            <p className="text-sm text-fg-muted">
              Defaults below — change them anytime in Settings.
            </p>
            <div className="bg-bg-soft rounded-xl border border-border divide-y divide-border">
              {DEFAULT_HOTKEYS.map((hk) => (
                <div key={hk.action} className="flex justify-between px-4 py-2.5">
                  <span className="text-sm text-fg">{hk.action}</span>
                  <kbd className="text-xs font-display bg-surface text-fg px-2 py-0.5 rounded border border-border">
                    {hk.chord}
                  </kbd>
                </div>
              ))}
            </div>
            <div className="flex justify-between items-center">
              <button
                className="text-sm text-fg-muted hover:text-fg"
                onClick={() => setStep('welcome')}
              >
                ← back
              </button>
              <PrimaryButton onClick={() => setStep('library')}>next</PrimaryButton>
            </div>
          </div>
        )}

        {step === 'library' && (
          <div className="space-y-5">
            <h2 className="font-display text-xl">💾 library location</h2>
            <p className="text-sm text-fg-muted">
              Your captures, clipboard history, and settings are stored locally. No cloud,
              no servers, no telemetry.
            </p>
            <div className="bg-bg-soft rounded-xl border border-border px-4 py-3">
              <div className="font-display text-[10px] uppercase tracking-wider text-fg-muted">
                Storage location
              </div>
              <div className="text-sm text-fg font-mono">%APPDATA%/snapper-keeper/</div>
            </div>
            <div className="flex justify-between items-center">
              <button
                className="text-sm text-fg-muted hover:text-fg"
                onClick={() => setStep('hotkeys')}
              >
                ← back
              </button>
              <PrimaryButton onClick={() => setStep('done')}>next</PrimaryButton>
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="text-center space-y-5">
            <h2 className="font-display text-2xl holo-shimmer bg-clip-text text-transparent">
              all set!
            </h2>
            <p className="text-sm text-fg-muted">
              Try pressing <kbd className="font-display text-[10px] bg-bg-soft border border-border px-1.5 py-0.5 rounded">Ctrl+Shift+4</kbd> for region capture, or{' '}
              <kbd className="font-display text-[10px] bg-bg-soft border border-border px-1.5 py-0.5 rounded">Ctrl+Shift+V</kbd> for clipboard history.
            </p>
            <PrimaryButton onClick={finish}>start using snapper-keeper</PrimaryButton>
          </div>
        )}
      </div>
    </div>
  );
}
