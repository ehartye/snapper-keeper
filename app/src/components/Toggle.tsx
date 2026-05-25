interface ToggleProps {
  value: boolean;
  onChange: (next: boolean) => void;
}

// Pill: 44×22 (w-11 h-[22px] — wider so the thumb has clean travel).
// Thumb: 16×16 (w-4 h-4). Off: thumb at x=2; on: x=(100% - 18px).
export function Toggle({ value, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      className={`w-11 h-[22px] rounded-full relative transition-colors border border-border ${
        value ? 'bg-primary' : 'bg-surface-2'
      }`}
      onClick={() => onChange(!value)}
    >
      <span
        className="absolute top-[2px] w-4 h-4 rounded-full bg-bg transition-[left] duration-150"
        style={{ left: value ? 'calc(100% - 18px)' : '2px' }}
      />
    </button>
  );
}
