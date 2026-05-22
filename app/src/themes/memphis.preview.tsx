/** Mockup of the .menu-divider rule defined in memphis.css. */
import type { DividerPreviewProps } from '../lib/theme';

function MemphisDividerPreview({ mode, preview }: DividerPreviewProps) {
  const fg = mode === 'dark' ? preview.fgDark : preview.fgLight;
  const stroke = encodeURIComponent(fg);
  return (
    <div
      style={{
        height: 12,
        backgroundImage: `url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 12' preserveAspectRatio='none'><path d='M0 6 Q 12.5 0 25 6 T 50 6 T 75 6 T 100 6' stroke='${stroke}' stroke-width='2.5' fill='none' /></svg>")`,
        backgroundRepeat: 'repeat-x',
        backgroundSize: '50px 12px',
      }}
    />
  );
}

export default MemphisDividerPreview;
