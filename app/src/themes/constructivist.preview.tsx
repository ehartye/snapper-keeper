/** Mockup of the .menu-divider rule defined in constructivist.css.
 *  10x10 red square tilted 15° + 2px horizontal bar — direct Lissitzky
 *  "Beat the Whites with the Red Wedge" composition. */
import type { DividerPreviewProps } from '../lib/theme';

function ConstructivistDividerPreview({ mode, preview }: DividerPreviewProps) {
  const fg = mode === 'dark' ? preview.fgDark : preview.fgLight;
  const primary = preview.swatches[0]!;
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
}

export default ConstructivistDividerPreview;
