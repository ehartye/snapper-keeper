import type { ClipboardItem } from '@snk/clipboard';

interface Props {
  item: ClipboardItem;
  index: number;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

function timeAgo(ms: number): string {
  const sec = Math.floor((Date.now() - ms) / 1000);
  if (sec < 60) return 'just now';
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
  return `${Math.floor(sec / 86400)}d ago`;
}

export function ClipboardPopupItem({ item, index, isSelected, onSelect }: Props) {
  const preview =
    item.kind === 'text'
      ? (item.text_content ?? '').slice(0, 120)
      : '[image]';

  return (
    <button
      onClick={() => onSelect(item.id)}
      className={`w-full text-left px-3 py-2 flex items-start gap-2 ${
        isSelected ? 'bg-blue-600/30' : 'hover:bg-slate-800'
      }`}
    >
      <span className="text-[10px] text-slate-500 w-4 shrink-0 text-right pt-0.5">
        {index < 9 ? index + 1 : ''}
      </span>
      <span className="text-xs text-slate-500 w-4 shrink-0 pt-0.5">
        {item.kind === 'text' ? 'T' : 'I'}
      </span>
      <div className="flex-1 min-w-0">
        <div className="text-xs text-slate-200 truncate">{preview}</div>
        <div className="text-[10px] text-slate-500 truncate">
          {item.source_app ?? 'unknown'} · {timeAgo(item.created_at)}
          {item.pinned ? ' · pinned' : ''}
        </div>
      </div>
    </button>
  );
}
