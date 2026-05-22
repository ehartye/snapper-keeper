/** Mockup of the .menu-divider rule defined in robotic.css. */
import type { DividerPreviewComponent } from '../lib/theme';

const RoboticDividerPreview: DividerPreviewComponent = ({ mode, preview }) => {
  const accent = preview.swatches[1]!;
  const muted = mode === 'dark' ? preview.mutedDark : preview.mutedLight;
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        gap: 6,
        height: 12,
        fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
        fontSize: 8,
        letterSpacing: '0.5px',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
      }}
    >
      <span style={{ color: accent }}>0x00FF</span>
      <span style={{ color: muted }}>41 6C 6C 0A 2D 54 41 47 53</span>
    </div>
  );
};

export default RoboticDividerPreview;
