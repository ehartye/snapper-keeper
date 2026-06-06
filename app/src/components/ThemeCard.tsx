import { THEME_FAMILIES, familyOf, type ThemeId } from '../lib/theme';

/**
 * Rich theme preview card — shows the family's personality (background, shape,
 * color swatches, display/body fonts, tagline, and the colocated divider
 * mockup) rather than just a name. Shared by the Settings appearance grid and
 * the first-run wizard so the theme picker looks the same everywhere.
 */
export function ThemeCard({
  themeId,
  label,
  active,
  onSelect,
}: {
  themeId: ThemeId;
  label: string;
  active: boolean;
  onSelect: () => void;
}) {
  const family = familyOf(themeId);
  const isDark = themeId.endsWith('dark');
  const preview = THEME_FAMILIES[family].preview;
  const DividerPreview = THEME_FAMILIES[family].DividerPreview;

  const shapeClass = (() => {
    switch (preview.shape) {
      case 'round':
        return active ? 'rounded-2xl ring-2 ring-primary' : 'rounded-2xl border border-border';
      case 'memphis':
        return active ? 'memphis-card-accent' : 'memphis-card';
      case 'terminal':
        return active
          ? 'rounded-none ring-2 ring-amber-500 border border-amber-700'
          : 'rounded-none border border-amber-700';
      case 'card':
        return active
          ? 'rounded-none ring-4 ring-[#18181b] border-2 border-[#18181b]'
          : 'rounded-none border-2 border-[#18181b]';
    }
  })();

  const tagline = THEME_FAMILIES[family].tagline;
  const fg = isDark ? preview.fgDark : preview.fgLight;
  const muted = isDark ? preview.mutedDark : preview.mutedLight;

  return (
    <button
      data-testid="theme-card"
      onClick={onSelect}
      className={`group relative text-left p-4 transition-transform hover:-translate-y-0.5 ${shapeClass}`}
      style={{ background: isDark ? preview.bgDark : preview.bgLight }}
    >
      <div className="flex gap-1.5 mb-3">
        {preview.swatches.map((c) => (
          <span
            key={c}
            className={`block ${preview.swatchShape === 'round' ? 'rounded-full' : ''}`}
            style={{
              width: 20,
              height: 20,
              background: c,
              border: preview.swatchShape === 'square' ? `2px solid ${fg}` : 'none',
            }}
          />
        ))}
      </div>

      <div
        className="text-sm leading-tight"
        style={{
          fontFamily: preview.displayFont,
          color: fg,
          letterSpacing: preview.shape === 'card' ? '0.08em' : undefined,
          textTransform: preview.shape === 'card' ? 'uppercase' : undefined,
        }}
      >
        {label}
      </div>

      <div
        className="text-[10px] mt-0.5 uppercase tracking-wider"
        style={{ fontFamily: preview.bodyFont, color: muted, opacity: 0.7 }}
      >
        {isDark ? 'dark' : 'light'}
      </div>

      <div
        className="text-[11px] mt-2 leading-snug italic"
        style={{ fontFamily: preview.bodyFont, color: muted }}
      >
        {tagline}
      </div>

      {/* Divider preview — colocated mockup at themes/<family>.preview.tsx.
          NOT the real .menu-divider because that's anchored to
          html[data-theme=X] and would inherit the active document theme
          inside the card. */}
      <div
        className="mt-3 px-2 py-1.5"
        style={{
          background: isDark ? preview.bgDark : preview.bgLight,
          filter: 'brightness(0.96)',
        }}
        aria-hidden
      >
        <DividerPreview mode={isDark ? 'dark' : 'light'} preview={preview} />
      </div>
    </button>
  );
}
