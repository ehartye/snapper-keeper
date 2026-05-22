import { describe, it, expect } from 'vitest';

import { captureAssetUrl } from './assetUrl';

describe('captureAssetUrl', () => {
  it('joins root and relative with a forward slash', () => {
    const url = captureAssetUrl('/data', 'captures/2026/05/x.png');
    expect(url).toBe('asset:///data/captures/2026/05/x.png');
  });

  it('strips trailing slashes from root', () => {
    expect(captureAssetUrl('/data/', 'a.png')).toBe('asset:///data/a.png');
    expect(captureAssetUrl('/data///', 'a.png')).toBe('asset:///data/a.png');
  });

  it('normalises backslashes in the relative path', () => {
    expect(
      captureAssetUrl('/data', 'captures\\2026\\05\\x.png'),
    ).toBe('asset:///data/captures/2026/05/x.png');
  });

  it('handles a root with a Windows-style trailing backslash', () => {
    expect(captureAssetUrl('C:\\data\\', 'x.png')).toBe('asset://C:\\data/x.png');
  });
});
