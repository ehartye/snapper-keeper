function isMacPlatform() {
  if (typeof navigator === 'undefined') return false;
  return /Mac|iPhone|iPad|iPod/.test(navigator.platform) || /\bMac OS\b/.test(navigator.userAgent);
}

export function formatShortcutForPlatform(chord: string, mac = isMacPlatform()) {
  if (!mac) return chord.replace(/\bCmdOrCtrl\b/g, 'Ctrl');
  return chord.replace(/\bCmdOrCtrl\b/g, 'Cmd').replace(/\bCtrl\b/g, 'Cmd');
}
