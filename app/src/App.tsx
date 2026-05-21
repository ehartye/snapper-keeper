import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';

import { LibraryWindow } from './windows/library/LibraryWindow';
import { CaptureOverlay } from './windows/capture-overlay/CaptureOverlay';
import { CaptureToolbar } from './windows/capture-toolbar/CaptureToolbar';
import { AnnotateWindow } from './windows/annotate/AnnotateWindow';

function WindowRouter() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    setLabel(getCurrentWindow().label);
  }, []);

  if (!label) return null;

  switch (label) {
    case 'library':
      return <LibraryWindow />;
    case 'capture-overlay':
      return <CaptureOverlay />;
    case 'capture-toolbar':
      return <CaptureToolbar />;
    case 'annotate':
      return <AnnotateWindow />;
    default:
      return <div>Unknown window: {label}</div>;
  }
}

export default function App() {
  return <WindowRouter />;
}
