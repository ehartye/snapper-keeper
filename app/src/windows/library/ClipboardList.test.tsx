import { describe, it, expect, vi } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { ClipboardList } from './ClipboardList';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<ClipboardList />', () => {
  it('shows the empty state when no items exist', async () => {
    mockedInvoke.mockResolvedValue([]);
    renderWithQuery(<ClipboardList />);
    expect(await screen.findByText(/nothing copied yet/i)).toBeInTheDocument();
  });

  it('renders text items with truncated preview', async () => {
    mockedInvoke.mockResolvedValue([
      {
        id: 'cb-1',
        kind: 'text',
        text_content: 'hello world',
        file_path: null,
        source_window_title: null,
        sensitive: false,
        content_hash: 'a',
        source_app: 'Firefox',
        pinned: false,
        created_at: Date.now(),
      },
    ]);
    renderWithQuery(<ClipboardList />);
    expect(await screen.findByText(/hello world/)).toBeInTheDocument();
    expect(screen.getByText(/Firefox/)).toBeInTheDocument();
  });

  it('writes to the system clipboard when a row is clicked, then flashes', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    // navigator.clipboard is a read-only getter; defineProperty around it.
    Object.defineProperty(window.navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });

    mockedInvoke.mockResolvedValue([
      {
        id: 'cb-1',
        kind: 'text',
        text_content: 'copied text',
        file_path: null,
        source_window_title: null,
        sensitive: false,
        content_hash: 'a',
        source_app: 'Firefox',
        pinned: false,
        created_at: Date.now(),
      },
    ]);
    renderWithQuery(<ClipboardList />);
    const row = await screen.findByRole('button', { name: /copied text/i });
    await act(async () => {
      fireEvent.click(row);
    });
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('copied text');
    });
    expect(await screen.findByText(/Copied/i)).toBeInTheDocument();
  });

  it('shows the "images can\'t be re-copied" notice when clicking an image row', async () => {
    mockedInvoke.mockResolvedValue([
      {
        id: 'cb-2',
        kind: 'image',
        text_content: null,
        file_path: 'clip/x.png',
        source_window_title: null,
        sensitive: false,
        content_hash: 'b',
        source_app: 'Photos',
        pinned: false,
        created_at: Date.now(),
      },
    ]);
    renderWithQuery(<ClipboardList />);
    const row = await screen.findByRole('button', { name: /image/i });
    await act(async () => {
      fireEvent.click(row);
    });
    expect(await screen.findByText(/can't be re-copied/i)).toBeInTheDocument();
  });

  it('pin toggle calls toggle_clipboard_pin without re-firing the row copy', async () => {
    mockedInvoke.mockResolvedValue([
      {
        id: 'cb-1',
        kind: 'text',
        text_content: 'x',
        file_path: null,
        source_window_title: null,
        sensitive: false,
        content_hash: 'a',
        source_app: null,
        pinned: false,
        created_at: Date.now(),
      },
    ]);
    renderWithQuery(<ClipboardList />);
    await screen.findByText(/x/);
    const pin = screen.getByTitle('Pin');
    await act(async () => {
      fireEvent.click(pin);
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|toggle_clipboard_pin', {
        id: 'cb-1',
        pinned: true,
      });
    });
  });
});
