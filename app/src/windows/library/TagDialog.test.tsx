import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { TagDialog } from './TagDialog';
import { renderWithQuery } from '../../test/renderWithQuery';

describe('<TagDialog />', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it("renders nothing when open=false", () => {
    const { container } = renderWithQuery(<TagDialog open={false} onClose={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders existing tags when open', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_tags') {
        return Promise.resolve([
          { id: 't1', name: 'work', color: '#ff0000', created_at: 0 },
        ]);
      }
      return Promise.resolve(undefined);
    });
    renderWithQuery(<TagDialog open={true} onClose={vi.fn()} />);
    expect(await screen.findByText('work')).toBeInTheDocument();
  });

  it('creates a tag via create_tag', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|create_tag') {
        return Promise.resolve({ id: 't2', name: 'home', color: '#ef4444', created_at: 0 });
      }
      return Promise.resolve([]);
    });
    renderWithQuery(<TagDialog open={true} onClose={vi.fn()} />);

    const nameInput = screen.getByPlaceholderText(/tag name/i) as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: 'home' } });
    fireEvent.click(screen.getByText(/^create$/i));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|create_tag', {
        name: 'home',
        color: '#ef4444',
      });
    });
  });

  it("does NOT call create_tag when the name is empty/whitespace", () => {
    renderWithQuery(<TagDialog open={true} onClose={vi.fn()} />);
    fireEvent.click(screen.getByText(/^create$/i));
    expect(invoke).not.toHaveBeenCalledWith(
      'plugin:snk-library|create_tag',
      expect.anything(),
    );
  });

  it('confirms before deleting and routes through delete_tag', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_tags') {
        return Promise.resolve([{ id: 't1', name: 'work', color: '#ff0000', created_at: 0 }]);
      }
      return Promise.resolve(undefined);
    });
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderWithQuery(<TagDialog open={true} onClose={vi.fn()} />);

    await screen.findByText('work');
    fireEvent.click(screen.getByText(/^delete$/i));

    await waitFor(() => {
      expect(confirm).toHaveBeenCalled();
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|delete_tag', { id: 't1' });
    });
    confirm.mockRestore();
  });

  it('switches into edit mode and uses update_tag', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-library|list_tags') {
        return Promise.resolve([{ id: 't1', name: 'work', color: '#ff0000', created_at: 0 }]);
      }
      return Promise.resolve(undefined);
    });
    renderWithQuery(<TagDialog open={true} onClose={vi.fn()} />);

    await screen.findByText('work');
    fireEvent.click(screen.getByText(/^edit$/i));
    expect(screen.getByText(/edit tag/i)).toBeInTheDocument();

    const input = screen.getByPlaceholderText(/tag name/i) as HTMLInputElement;
    expect(input.value).toBe('work');
    fireEvent.change(input, { target: { value: 'jobs' } });
    fireEvent.click(screen.getByText(/^update$/i));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|update_tag', {
        id: 't1',
        name: 'jobs',
        color: '#ff0000',
      });
    });
  });

  it('Close button calls onClose', () => {
    const onClose = vi.fn();
    renderWithQuery(<TagDialog open={true} onClose={onClose} />);
    fireEvent.click(screen.getByText(/^close$/i));
    expect(onClose).toHaveBeenCalled();
  });
});
