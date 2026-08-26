import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor, act, fireEvent } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { ClipboardPopup } from './ClipboardPopup';
import { useClipboardPopupStore } from './store';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

function seedItems(n: number) {
  return Array.from({ length: n }, (_, i) => ({
    id: `cb-${i + 1}`,
    kind: 'text' as const,
    text_content: `item ${i + 1}`,
    file_path: null,
    source_window_title: null,
    content_hash: `${i}`,
    source_app: 'Test',
    pinned: false,
    created_at: Date.now(),
  }));
}

describe('<ClipboardPopup />', () => {
  beforeEach(() => {
    useClipboardPopupStore.getState().reset();
    useClipboardPopupStore.getState().setItems(seedItems(5));
    // Default invoke → an empty array so anything that re-fetches the list
    // (e.g. after Ctrl+P toggles a pin) doesn't end up with `undefined`.
    mockedInvoke.mockReset().mockResolvedValue([]);
  });

  it('renders the seeded items', () => {
    renderWithQuery(<ClipboardPopup />);
    expect(screen.getByText('item 1')).toBeInTheDocument();
    expect(screen.getByText('item 5')).toBeInTheDocument();
  });

  it("shows 'nothing copied yet' when the list is empty", () => {
    useClipboardPopupStore.getState().setItems([]);
    renderWithQuery(<ClipboardPopup />);
    expect(screen.getByText(/nothing copied yet/i)).toBeInTheDocument();
  });

  it('arrow-down moves selection through the list', () => {
    renderWithQuery(<ClipboardPopup />);
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(0);
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(1);
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(2);
  });

  it("arrow-up doesn't move past the first item", () => {
    renderWithQuery(<ClipboardPopup />);
    fireEvent.keyDown(window, { key: 'ArrowUp' });
    expect(useClipboardPopupStore.getState().selectedIndex).toBe(0);
  });

  it('Enter pastes the currently-selected item via paste_item', async () => {
    renderWithQuery(<ClipboardPopup />);
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    await act(async () => {
      fireEvent.keyDown(window, { key: 'Enter' });
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|paste_item', {
        id: 'cb-2',
        targetApp: undefined,
      });
    });
  });

  it('plain digit 1-9 pastes that row when filter is empty', async () => {
    renderWithQuery(<ClipboardPopup />);
    await act(async () => {
      fireEvent.keyDown(window, { key: '3' });
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|paste_item', {
        id: 'cb-3',
        targetApp: undefined,
      });
    });
  });

  it('plain digit is suppressed when the filter input has content', async () => {
    useClipboardPopupStore.getState().setFilter('match');
    renderWithQuery(<ClipboardPopup />);
    // Focus the filter input so the suppression branch fires.
    const input = screen.getByPlaceholderText(/filter/i) as HTMLInputElement;
    input.focus();

    fireEvent.keyDown(window, { key: '2' });
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      'plugin:snk-clipboard|paste_item',
      expect.anything(),
    );
  });

  it('Ctrl+P toggles the pinned flag on the selected item', async () => {
    renderWithQuery(<ClipboardPopup />);
    await act(async () => {
      fireEvent.keyDown(window, { key: 'p', ctrlKey: true });
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-library|toggle_clipboard_pin',
        { id: 'cb-1', pinned: true },
      );
    });
  });

  it('surfaces an accessibility banner when paste reports accessibility-required', async () => {
    mockedInvoke.mockReset().mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-clipboard|paste_item') {
        return Promise.reject({ kind: 'accessibility-required' });
      }
      return Promise.resolve([]);
    });
    renderWithQuery(<ClipboardPopup />);
    await act(async () => {
      fireEvent.keyDown(window, { key: 'Enter' });
    });
    await waitFor(() => {
      expect(screen.getByText(/accessibility needed to paste/i)).toBeInTheDocument();
    });
    // The "open settings" affordance invokes the deep-link command.
    await act(async () => {
      fireEvent.click(screen.getByText(/open settings/i));
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-clipboard|open_accessibility_settings',
      );
    });
  });

  it('passes the preserved target app back into paste_item after popup show', async () => {
    let showHandler: ((e: { payload: { targetApp: unknown } }) => void) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'clipboard-popup:show') {
        showHandler = handler as typeof showHandler;
      }
      return Promise.resolve(() => {});
    });
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_clipboard_items') {
        return Promise.resolve(seedItems(5));
      }
      if (cmd === 'plugin:snk-clipboard|clipboard_permission_status') {
        return Promise.resolve({ accessibility_granted: true });
      }
      return Promise.resolve([]);
    });

    renderWithQuery(<ClipboardPopup />);
    await waitFor(() => expect(showHandler).not.toBeNull());

    await act(async () => {
      showHandler!({
        payload: {
          targetApp: {
            identifier: 'com.apple.TextEdit',
            display_name: 'TextEdit',
            kind: 'macos_bundle_id',
          },
        },
      });
    });

    await act(async () => {
      fireEvent.keyDown(window, { key: 'Enter' });
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-clipboard|paste_item', {
        id: 'cb-1',
        targetApp: {
          identifier: 'com.apple.TextEdit',
          display_name: 'TextEdit',
          kind: 'macos_bundle_id',
        },
      });
    });
  });

  it('Escape dismisses the popup and resets store state', async () => {
    renderWithQuery(<ClipboardPopup />);
    await act(async () => {
      fireEvent.keyDown(window, { key: 'Escape' });
    });
    await waitFor(() => {
      expect(useClipboardPopupStore.getState().items.length).toBe(0);
    });
  });
});
