import type { ClipboardItem } from '@snk/clipboard';

import { formatTimeAgo } from '../../lib/formatTimeAgo';

interface Props {
  item: ClipboardItem;
  index: number;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

export function ClipboardPopupItem({ item, index, isSelected, onSelect }: Props) {
  const preview =
    item.kind === 'text'
      ? (item.text_content ?? '').slice(0, 120)
      : '[image]';

  return (
    <button
      onClick={() => onSelect(item.id)}
      className={`w-full text-left px-3 py-2 flex items-start gap-2 border-b border-border last:border-0 ${
        isSelected ? 'bg-primary/15' : 'hover:bg-surface-2'
      }`}
    >
      <span className="text-[10px] font-display text-accent-2 w-4 shrink-0 text-right pt-0.5">
        {index < 9 ? index + 1 : ''}
      </span>
      <span className="text-xs font-display text-fg-muted w-4 shrink-0 pt-0.5">
        {item.kind === 'text' ? 'T' : 'I'}
      </span>
      <div className="flex-1 min-w-0">
        <div className="text-xs text-fg truncate">{preview}</div>
        <div className="text-[10px] text-fg-muted truncate">
          {item.source_app ?? 'unknown'} · {formatTimeAgo(item.created_at)}
          {item.pinned ? ' · ★' : ''}
        </div>
      </div>
    </button>
  );
}
