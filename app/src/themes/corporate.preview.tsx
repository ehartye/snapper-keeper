/** Mockup of the .menu-divider rule defined in corporate.css. */
import type { DividerPreviewComponent } from '../lib/theme';

const CorporateDividerPreview: DividerPreviewComponent = ({ mode, preview }) => {
  const isDark = mode === 'dark';
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const muted = isDark ? preview.mutedDark : preview.mutedLight;
  const bg = isDark ? preview.bgDark : preview.bgLight;
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: 12,
        backgroundImage: `linear-gradient(to right, ${fg}, ${fg})`,
        backgroundSize: '100% 1px',
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat',
      }}
    >
      <span
        style={{
          background: bg,
          padding: '0 6px',
          fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
          fontSize: 8,
          fontWeight: 600,
          letterSpacing: '1px',
          color: muted,
          textTransform: 'uppercase',
        }}
      >
        № I
      </span>
    </div>
  );
};

export default CorporateDividerPreview;
