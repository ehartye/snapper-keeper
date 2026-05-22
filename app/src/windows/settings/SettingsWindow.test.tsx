import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { SettingsWindow } from './SettingsWindow';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<SettingsWindow />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue(null);
  });

  it('renders the Settings header and Appearance + Capture + Clipboard + OCR sections', async () => {
    renderWithQuery(<SettingsWindow />);
    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('Capture')).toBeInTheDocument();
    expect(screen.getByText('Clipboard')).toBeInTheDocument();
    expect(screen.getByText('OCR')).toBeInTheDocument();
  });

  it('lists all 8 theme cards', async () => {
    renderWithQuery(<SettingsWindow />);
    // Each ThemeCard's label is the family name (first chunk before — )
    for (const label of [
      'Holographic Dreamcore',
      'Memphis Machine',
      'Mr Robotic',
      'Corporate Overlord',
    ]) {
      expect(screen.getAllByText(label).length).toBeGreaterThanOrEqual(2);
    }
  });

  it('clicking a theme card persists the new theme and applies it', async () => {
    renderWithQuery(<SettingsWindow />);
    const cards = await screen.findAllByText('Memphis Machine');
    fireEvent.click(cards[0]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'theme',
        value: expect.stringContaining('memphis'),
      });
    });
  });

  it('changing the capture format calls setSetting', async () => {
    renderWithQuery(<SettingsWindow />);
    const select = await screen.findByDisplayValue(/png/i);
    await act(async () => {
      fireEvent.change(select, { target: { value: 'jpg' } });
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'capture.format',
        value: 'jpg',
      });
    });
  });

  it('toggling Auto-copy persists the new boolean', async () => {
    mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'plugin:snk-library|get_setting') {
        const key = (args as { key: string }).key;
        if (key === 'capture.auto_copy') return Promise.resolve(true);
        return Promise.resolve(null);
      }
      return Promise.resolve(undefined);
    });
    const { container } = renderWithQuery(<SettingsWindow />);
    // Wait for the initial query so the toggle reflects 'true'.
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalled());

    // The Toggle is the *only* w-11 h-[22px] pill button on the page
    // belonging to the Capture section's auto_copy row. Find it by the row
    // text and walk the DOM up to the containing SettingRow.
    const rowLabel = await screen.findByText(/Auto-copy/i);
    const row = rowLabel.closest('div.flex')!;
    const toggle = row.querySelector('button');
    expect(toggle).toBeTruthy();

    await act(async () => fireEvent.click(toggle!));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'capture.auto_copy',
        value: false,
      });
    });
  });
});
