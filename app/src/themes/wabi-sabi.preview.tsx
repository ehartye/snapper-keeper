/** Mockup of the .menu-divider rule defined in wabi-sabi.css.
 *  The kanji 章 ("chapter/section") on a vermillion hanko stamp sits over
 *  a wandering brush stroke — irregularity is the design language. */
import type { DividerPreviewProps } from '../lib/theme';

function WabiSabiDividerPreview({ mode, preview }: DividerPreviewProps) {
  const isDark = mode === 'dark';
  const primary = preview.swatches[0]!; // vermillion
  const bg = isDark ? preview.bgDark : preview.bgLight;
  const brushColor = encodeURIComponent(isDark ? '#c8a878' : '#2a1f1a');
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: 14,
        backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 4' preserveAspectRatio='none'><path d='M2 2.2 Q 22 0.8 50 2.5 Q 78 4.2 98 2' stroke='${brushColor}' stroke-width='0.6' fill='none' stroke-linecap='round' opacity='0.55' /></svg>")`,
        backgroundSize: '100% 4px',
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat',
      }}
    >
      <span
        style={{
          background: primary,
          color: bg,
          fontFamily:
            "'Shippori Mincho', 'Yu Mincho', 'Hiragino Mincho ProN', 'Noto Serif JP', serif",
          fontWeight: 800,
          fontSize: 10,
          lineHeight: 1,
          padding: '2px 3px',
          borderRadius: 1,
          transform: 'rotate(-3deg)',
          boxShadow: 'inset 0 0 2px rgba(0,0,0,0.4)',
        }}
      >
        章
      </span>
    </div>
  );
}

export default WabiSabiDividerPreview;
