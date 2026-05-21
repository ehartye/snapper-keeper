import { useState } from 'react';

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

export function FirstRunWizard({ onComplete }: Props) {
  const [step, setStep] = useState<Step>('welcome');

  const finish = async () => {
    await setSetting('firstrun.completed', true);
    onComplete();
  };

  return (
    <div className="fixed inset-0 bg-slate-950 flex items-center justify-center z-50">
      <div className="max-w-md w-full mx-4">
        {step === 'welcome' && (
          <div className="text-center space-y-4">
            <h1 className="text-xl font-semibold text-slate-100">Welcome to snapper-keeper</h1>
            <p className="text-sm text-slate-400">
              Screen capture with OCR search, plus clipboard history with instant paste.
              Let&apos;s get you set up.
            </p>
            <button
              className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
              onClick={() => setStep('hotkeys')}
            >
              Get started
            </button>
          </div>
        )}

        {step === 'hotkeys' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">Keyboard shortcuts</h2>
            <p className="text-sm text-slate-400">
              These are the default hotkeys. You can change them anytime in Settings.
            </p>
            <div className="bg-slate-900 rounded-lg border border-slate-800 divide-y divide-slate-800">
              {DEFAULT_HOTKEYS.map((hk) => (
                <div key={hk.action} className="flex justify-between px-4 py-2">
                  <span className="text-sm text-slate-200">{hk.action}</span>
                  <kbd className="text-xs bg-slate-800 text-slate-300 px-2 py-0.5 rounded border border-slate-700">
                    {hk.chord}
                  </kbd>
                </div>
              ))}
            </div>
            <div className="flex justify-between">
              <button
                className="text-sm text-slate-400 hover:text-slate-200"
                onClick={() => setStep('welcome')}
              >
                Back
              </button>
              <button
                className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
                onClick={() => setStep('library')}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {step === 'library' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">Library location</h2>
            <p className="text-sm text-slate-400">
              Your captures, clipboard history, and settings are stored locally.
              No cloud, no servers, no telemetry.
            </p>
            <div className="bg-slate-900 rounded-lg border border-slate-800 px-4 py-3">
              <div className="text-[10px] text-slate-500">Storage location</div>
              <div className="text-sm text-slate-200 font-mono">%APPDATA%/snapper-keeper/</div>
            </div>
            <div className="flex justify-between">
              <button
                className="text-sm text-slate-400 hover:text-slate-200"
                onClick={() => setStep('hotkeys')}
              >
                Back
              </button>
              <button
                className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
                onClick={() => setStep('done')}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="text-center space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">All set!</h2>
            <p className="text-sm text-slate-400">
              You&apos;re ready to go. Try pressing Ctrl+Shift+4 to capture a region,
              or Ctrl+Shift+V to open clipboard history.
            </p>
            <button
              className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
              onClick={finish}
            >
              Start using snapper-keeper
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
