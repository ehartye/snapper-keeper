import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { SearchBar } from './SearchBar';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<SearchBar />', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  it('debounces and queries searchLibrary 250ms after typing stops', async () => {
    mockedInvoke.mockResolvedValue([]);
    renderWithQuery(<SearchBar />);
    const input = screen.getByPlaceholderText(/search captures/i);

    fireEvent.change(input, { target: { value: 'hi' } });
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      'plugin:snk-library|search_library',
      expect.anything(),
    );

    await act(async () => {
      vi.advanceTimersByTime(260);
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|search_library', {
        query: 'hi',
        limit: 20,
      });
    });
  });

  it('shows results once the query resolves and lets you click one to open annotate', async () => {
    mockedInvoke.mockResolvedValue([
      { kind: 'capture', id: 'c1', rank: 1, snippet: 'match' },
    ]);
    renderWithQuery(<SearchBar />);

    const input = screen.getByPlaceholderText(/search captures/i);
    fireEvent.change(input, { target: { value: 'm' } });
    await act(async () => {
      vi.advanceTimersByTime(260);
    });

    const row = await screen.findByText(/match/);
    fireEvent.click(row);

    await waitFor(() => {
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('annotate');
    });
  });

  it('switches to the clipboard sidebar when a clipboard result is clicked', async () => {
    mockedInvoke.mockResolvedValue([
      { kind: 'clipboard', id: 'cb1', rank: 1, snippet: 'pasted' },
    ]);
    const onSelectClipboard = vi.fn();
    renderWithQuery(<SearchBar onSelectClipboard={onSelectClipboard} />);

    const input = screen.getByPlaceholderText(/search captures/i);
    fireEvent.change(input, { target: { value: 'p' } });
    await act(async () => {
      vi.advanceTimersByTime(260);
    });

    const row = await screen.findByText(/pasted/);
    fireEvent.click(row);
    expect(onSelectClipboard).toHaveBeenCalledTimes(1);
  });

  it('arrow keys move the active row and Enter opens it', async () => {
    mockedInvoke.mockResolvedValue([
      { kind: 'capture', id: 'c1', rank: 1, snippet: 'first' },
      { kind: 'capture', id: 'c2', rank: 0.9, snippet: 'second' },
    ]);
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderWithQuery(<SearchBar />);

    const input = screen.getByPlaceholderText(/search captures/i);
    await user.type(input, 'x');
    await act(async () => {
      vi.advanceTimersByTime(260);
    });
    await screen.findByText(/first/);

    await user.keyboard('{ArrowDown}');
    await user.keyboard('{Enter}');

    await waitFor(() => {
      // Second item (c2) should have been opened via annotate.
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('annotate');
    });
  });

  it('shows a "No results" message when the query returns nothing', async () => {
    mockedInvoke.mockResolvedValue([]);
    renderWithQuery(<SearchBar />);
    const input = screen.getByPlaceholderText(/search captures/i);
    fireEvent.change(input, { target: { value: 'zzz' } });
    await act(async () => {
      vi.advanceTimersByTime(260);
    });

    await waitFor(() => {
      expect(screen.getByText(/No results/i)).toBeInTheDocument();
    });
  });

  it('clear button empties the input and hides results', async () => {
    mockedInvoke.mockResolvedValue([
      { kind: 'capture', id: 'c1', rank: 1, snippet: 'match' },
    ]);
    renderWithQuery(<SearchBar />);
    const input = screen.getByPlaceholderText(/search captures/i);
    fireEvent.change(input, { target: { value: 'h' } });
    await act(async () => {
      vi.advanceTimersByTime(260);
    });
    await screen.findByText(/match/);

    fireEvent.click(screen.getByText(/clear/));
    expect((input as HTMLInputElement).value).toBe('');
  });

  it('Escape clears the input when there is text', async () => {
    renderWithQuery(<SearchBar />);
    const input = screen.getByPlaceholderText(/search captures/i);
    fireEvent.change(input, { target: { value: 'abc' } });
    fireEvent.keyDown(input, { key: 'Escape' });
    expect((input as HTMLInputElement).value).toBe('');
  });
});
