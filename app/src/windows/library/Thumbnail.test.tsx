import { describe, it, expect, vi } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import type { Capture } from '@snk/library';

import { Thumbnail } from './Thumbnail';
import { renderWithQuery } from '../../test/renderWithQuery';

const baseCapture: Capture = {
  id: 'cap-1',
  file_path: 'captures/2026/05/x.png',
  annotated_path: null,
  width: 1920,
  height: 1080,
  source_app: 'Firefox',
  source_window_title: 'github',
  monitor: 'Monitor 0',
  created_at: Date.UTC(2026, 4, 22, 12, 0, 0),
  deleted_at: null,
  pinned: false,
};

describe('<Thumbnail />', () => {
  it('renders dimensions, monitor, and source app', () => {
    renderWithQuery(<Thumbnail capture={baseCapture} src="asset:///x.png" />);
    expect(screen.getByText(/1920×1080/)).toBeInTheDocument();
    expect(screen.getByText(/Monitor 0/)).toBeInTheDocument();
    expect(screen.getByText(/Firefox/)).toBeInTheDocument();
  });

  it('shows an "annotated" badge when annotated_path is set', () => {
    renderWithQuery(
      <Thumbnail
        capture={{ ...baseCapture, annotated_path: 'x.annotated.png' }}
        src="asset:///x.png"
      />,
    );
    expect(screen.getByText('annotated')).toBeInTheDocument();
  });

  it('opens the annotate window when the card body is clicked (not in trash)', async () => {
    renderWithQuery(<Thumbnail capture={baseCapture} src="asset:///x.png" />);
    fireEvent.click(screen.getByAltText('Capture cap-1'));

    await waitFor(() => {
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('annotate');
    });
  });

  it('confirms before permanently deleting a trashed capture on body click', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderWithQuery(<Thumbnail capture={baseCapture} src="asset:///x.png" inTrash />);

    fireEvent.click(screen.getByAltText('Capture cap-1'));
    await waitFor(() => {
      expect(confirm).toHaveBeenCalled();
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|hard_delete_capture', {
        id: 'cap-1',
      });
    });
    confirm.mockRestore();
  });

  it("does NOT delete when the user cancels the permanent-delete prompt", async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    renderWithQuery(<Thumbnail capture={baseCapture} src="asset:///x.png" inTrash />);

    fireEvent.click(screen.getByAltText('Capture cap-1'));
    expect(confirm).toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith(
      'plugin:snk-library|hard_delete_capture',
      expect.anything(),
    );
    confirm.mockRestore();
  });

  it('soft-deletes via the corner X (normal view)', async () => {
    const { container } = renderWithQuery(
      <Thumbnail capture={baseCapture} src="asset:///x.png" />,
    );
    const delBtn = Array.from(container.querySelectorAll('button')).find(
      (b) => b.getAttribute('title') === 'Delete',
    )!;
    fireEvent.click(delBtn);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|soft_delete_capture', {
        id: 'cap-1',
      });
    });
  });

  it('toggles pinned via the pin button in the corner', async () => {
    const { container } = renderWithQuery(
      <Thumbnail capture={baseCapture} src="asset:///x.png" />,
    );
    const pinBtn = Array.from(container.querySelectorAll('button')).find(
      (b) => b.getAttribute('title') === 'Pin',
    )!;
    fireEvent.click(pinBtn);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_capture_pinned', {
        id: 'cap-1',
        pinned: true,
      });
    });
  });

  it('shows Unpin title and the right pinned→false call when already pinned', async () => {
    const { container } = renderWithQuery(
      <Thumbnail
        capture={{ ...baseCapture, pinned: true }}
        src="asset:///x.png"
      />,
    );
    const pinBtn = Array.from(container.querySelectorAll('button')).find(
      (b) => b.getAttribute('title') === 'Unpin',
    )!;
    fireEvent.click(pinBtn);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_capture_pinned', {
        id: 'cap-1',
        pinned: false,
      });
    });
  });

  it('hides the pin button entirely while in trash', () => {
    const { container } = renderWithQuery(
      <Thumbnail capture={baseCapture} src="asset:///x.png" inTrash />,
    );
    const pinBtn = Array.from(container.querySelectorAll('button')).find(
      (b) => (b.getAttribute('title') ?? '').toLowerCase().includes('pin'),
    );
    expect(pinBtn).toBeUndefined();
  });

  it('opens the tag context menu on right-click', async () => {
    renderWithQuery(<Thumbnail capture={baseCapture} src="asset:///x.png" />);
    fireEvent.contextMenu(screen.getByAltText('Capture cap-1'));
    expect(await screen.findByText('Tags')).toBeInTheDocument();
  });
});
