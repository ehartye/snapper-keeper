import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';

import { ClipboardPopupItem } from './ClipboardPopupItem';
import type { ClipboardItem } from '@snk/clipboard';

const baseItem: ClipboardItem = {
  id: 'cb-1',
  content_hash: 'abc',
  kind: 'text',
  text_content: 'hello world',
  file_path: null,
  source_app: 'Firefox',
  source_window_title: null,
  pinned: false,
  created_at: Date.now(),
};

describe('<ClipboardPopupItem />', () => {
  it('renders the text preview, source app, and the 1-based index', () => {
    render(
      <ClipboardPopupItem
        item={baseItem}
        index={0}
        isSelected={false}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText('hello world')).toBeInTheDocument();
    expect(screen.getByText(/Firefox/)).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('shows the type letter for text items', () => {
    render(
      <ClipboardPopupItem item={baseItem} index={0} isSelected={false} onSelect={vi.fn()} />,
    );
    expect(screen.getByText('T')).toBeInTheDocument();
  });

  it('shows [image] preview and the I letter for image items', () => {
    const img: ClipboardItem = {
      ...baseItem,
      id: 'cb-2',
      kind: 'image',
      text_content: null,
      file_path: 'clip/x.png',
    };
    render(<ClipboardPopupItem item={img} index={1} isSelected={false} onSelect={vi.fn()} />);
    expect(screen.getByText('[image]')).toBeInTheDocument();
    expect(screen.getByText('I')).toBeInTheDocument();
  });

  it('drops the index display beyond row 9', () => {
    const { container } = render(
      <ClipboardPopupItem item={baseItem} index={10} isSelected={false} onSelect={vi.fn()} />,
    );
    // Index 10 means the 11th row; the index cell should be empty.
    const idxCell = container.querySelector('span');
    expect(idxCell?.textContent).toBe('');
  });

  it('switches background to bg-primary when selected', () => {
    const { container } = render(
      <ClipboardPopupItem item={baseItem} index={0} isSelected={true} onSelect={vi.fn()} />,
    );
    const button = container.querySelector('button')!;
    expect(button.className).toContain('bg-primary');
  });

  it('annotates pinned items with a ★', () => {
    render(
      <ClipboardPopupItem
        item={{ ...baseItem, pinned: true }}
        index={0}
        isSelected={false}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText(/★/)).toBeInTheDocument();
  });

  it('calls onSelect(id) when clicked', () => {
    const onSelect = vi.fn();
    render(
      <ClipboardPopupItem item={baseItem} index={0} isSelected={false} onSelect={onSelect} />,
    );
    fireEvent.click(screen.getByRole('button'));
    expect(onSelect).toHaveBeenCalledWith('cb-1');
  });

  it("falls back to 'unknown' source when none is set", () => {
    render(
      <ClipboardPopupItem
        item={{ ...baseItem, source_app: null }}
        index={0}
        isSelected={false}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText(/unknown/)).toBeInTheDocument();
  });
});
