import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { Sidebar, type SidebarSelection } from './Sidebar';
import { renderWithQuery } from '../../test/renderWithQuery';

const ALL_SECTION: SidebarSelection = { type: 'captures', label: 'All', query: {} };

describe('<Sidebar />', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it('renders all the smart sections', () => {
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={vi.fn()} />);
    for (const label of ['All', 'Today', 'This Week', 'Pinned', 'Trash']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it('fires onSelect with the right query when a smart section is clicked', () => {
    const onSelect = vi.fn();
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('Pinned'));
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'captures',
        label: 'Pinned',
        query: expect.objectContaining({ pinned_only: true }),
      }),
    );
  });

  it('fires onSelect with deleted_only when Trash is clicked', () => {
    const onSelect = vi.fn();
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('Trash'));
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({
        query: expect.objectContaining({ deleted_only: true }),
      }),
    );
  });

  it('switches to clipboard view when Clipboard History is clicked', () => {
    const onSelect = vi.fn();
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('Clipboard History'));
    expect(onSelect).toHaveBeenCalledWith({ type: 'clipboard' });
  });

  it('opens the settings window when Settings is clicked', async () => {
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={vi.fn()} />);
    fireEvent.click(screen.getByText('Settings'));
    await waitFor(() => {
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('settings');
    });
  });

  it('highlights the active smart section', () => {
    renderWithQuery(
      <Sidebar
        selection={{ type: 'captures', label: 'Pinned', query: {} }}
        onSelect={vi.fn()}
      />,
    );
    const pinned = screen.getByText('Pinned').closest('button')!;
    expect(pinned.className).toContain('bg-primary');
  });

  it("shows 'No tags yet' when there are no tags", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_tags') return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={vi.fn()} />);
    expect(await screen.findByText(/no tags yet/i)).toBeInTheDocument();
  });

  it('renders tags from listTags and selects on click', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_tags') {
        return Promise.resolve([
          { id: 't1', name: 'work', color: '#ff0000', created_at: 0 },
        ]);
      }
      return Promise.resolve(undefined);
    });
    const onSelect = vi.fn();
    renderWithQuery(<Sidebar selection={ALL_SECTION} onSelect={onSelect} />);

    const tag = await screen.findByText('work');
    fireEvent.click(tag);
    await waitFor(() => {
      expect(onSelect).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'captures',
          label: 'work',
          query: { tag_id: 't1' },
        }),
      );
    });
  });
});
