/** Mockup of the .menu-divider rule defined in holo.css. Lives here so a
 *  future maintainer touching the divider visual updates both files in one
 *  place. Rendered inside the Settings theme picker card. */
import type { DividerPreviewProps } from '../lib/theme';

function HoloDividerPreview({ preview }: DividerPreviewProps) {
  const primary = preview.swatches[0]!;
  const accent2 = preview.swatches[1]!;
  const accent = preview.swatches[2]!;
  return (
    <div
      style={{
        height: 12,
        backgroundImage: `linear-gradient(to right, transparent 0%, ${accent} 25%, ${primary} 50%, ${accent2} 75%, transparent 100%)`,
        backgroundSize: '100% 2px',
        backgroundRepeat: 'no-repeat',
        backgroundPosition: 'center',
      }}
    />
  );
}

export default HoloDividerPreview;
