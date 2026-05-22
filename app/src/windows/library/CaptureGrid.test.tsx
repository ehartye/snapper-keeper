import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { CaptureGrid } from './CaptureGrid';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

function makeCapture(id: string) {
  return {
    id,
    file_path: `c/${id}.png`,
    annotated_path: null,
    width: 100,
    height: 100,
    source_app: null,
    source_window_title: null,
    monitor: null,
    created_at: Date.UTC(2026, 4, 22),
    deleted_at: null,
    pinned: false,
  };
}

describe('<CaptureGrid />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it('shows the loading state, then the empty state when no captures', async () => {
    mockedInvoke.mockResolvedValue([]);
    renderWithQuery(<CaptureGrid />);
    expect(await screen.findByText(/nothing yet/i)).toBeInTheDocument();
  });

  it('renders thumbnails for returned captures', async () => {
    mockedInvoke.mockResolvedValue([makeCapture('a'), makeCapture('b')]);
    renderWithQuery(<CaptureGrid />);
    expect(await screen.findByAltText('Capture a')).toBeInTheDocument();
    expect(screen.getByAltText('Capture b')).toBeInTheDocument();
  });

  it('shows the "trash is empty" empty state when no items + deleted_only', async () => {
    mockedInvoke.mockResolvedValue([]);
    renderWithQuery(<CaptureGrid query={{ deleted_only: true }} />);
    expect(await screen.findByText(/trash is empty/i)).toBeInTheDocument();
  });

  it('renders the Empty trash header when in trash with items', async () => {
    mockedInvoke.mockResolvedValue([makeCapture('a')]);
    renderWithQuery(<CaptureGrid query={{ deleted_only: true }} />);
    expect(await screen.findByText(/Empty trash/i)).toBeInTheDocument();
    expect(screen.getByText(/^Trash$/)).toBeInTheDocument();
  });

  it('Empty trash → user confirms → purge_trash is called', async () => {
    mockedInvoke.mockResolvedValue([makeCapture('a'), makeCapture('b')]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderWithQuery(<CaptureGrid query={{ deleted_only: true }} />);

    const btn = await screen.findByText(/Empty trash/i);
    fireEvent.click(btn);

    await waitFor(() => {
      expect(confirm).toHaveBeenCalled();
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|purge_trash');
    });
    confirm.mockRestore();
  });

  it('Empty trash → user cancels → purge_trash is NOT called', async () => {
    mockedInvoke.mockResolvedValue([makeCapture('a')]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    renderWithQuery(<CaptureGrid query={{ deleted_only: true }} />);

    const btn = await screen.findByText(/Empty trash/i);
    fireEvent.click(btn);

    expect(confirm).toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('plugin:snk-library|purge_trash');
    confirm.mockRestore();
  });
});
