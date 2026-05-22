/** Mockup of the .menu-divider rule defined in riso.css.
 *  Two parallel hairlines intentionally misregistered by 2px right + 2px
 *  down — pink + blue on cream, pink + cyan on black. */
import type { DividerPreviewComponent } from '../lib/theme';

const RisoDividerPreview: DividerPreviewComponent = ({ preview }) => {
  const primary = preview.swatches[0]!;
  const accent = preview.swatches[1]!;
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
};

export default RisoDividerPreview;
