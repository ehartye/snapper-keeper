function isMacPlatform() {
  if (typeof navigator === 'undefined') return false;
  const uaDataPlatform = (
    navigator as Navigator & { userAgentData?: { platform?: string } }
  ).userAgentData?.platform;
  return (
    /mac/i.test(uaDataPlatform ?? '') ||
    /Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
    /\bMac OS\b/.test(navigator.userAgent)
  );
}

export function formatShortcutForPlatform(chord: string, mac = isMacPlatform()) {
  if (!mac) return chord.replace(/\bCmdOrCtrl\b/g, 'Ctrl');
  return chord.replace(/\bCmdOrCtrl\b/g, 'Cmd').replace(/\bCtrl\b/g, 'Cmd');
}
