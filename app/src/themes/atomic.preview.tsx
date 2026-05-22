/** Mockup of the .menu-divider rule defined in atomic.css.
 *  4-point compass starburst in mustard/marigold, masked by the card bg
 *  to occlude the line behind it — Saul Bass mid-century mark. */
import type { DividerPreviewComponent } from '../lib/theme';

const AtomicDividerPreview: DividerPreviewComponent = ({ mode, preview }) => {
  const isDark = mode === 'dark';
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const bg = isDark ? preview.bgDark : preview.bgLight;
  const primary = preview.swatches[0]!;
  const starFill = encodeURIComponent(primary);
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: 14,
        backgroundImage: `linear-gradient(to right, ${fg}, ${fg})`,
        backgroundSize: '100% 1px',
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat',
      }}
    >
      <span
        style={{
          width: 12,
          height: 12,
          backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'><path d='M6 0.5 L7 5 L11.5 6 L7 7 L6 11.5 L5 7 L0.5 6 L5 5 Z' fill='${starFill}' /></svg>")`,
          backgroundSize: '12px 12px',
          backgroundColor: bg,
          backgroundRepeat: 'no-repeat',
          backgroundPosition: 'center',
          padding: '0 4px',
          boxSizing: 'content-box',
        }}
      />
    </div>
  );
};

export default AtomicDividerPreview;
