import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { ModalProvider } from '../../components/Modal';
import { SettingsWindow } from './SettingsWindow';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<SettingsWindow />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'plugin:snk-updater|get_update_status')
        return Promise.resolve({ kind: 'idle' });
      if (cmd === 'plugin:snk-updater|get_last_check_at')
        return Promise.resolve(null);
      if (cmd === 'plugin:snk-library|get_setting') {
        const key = (args as { key: string }).key;
        if (key === 'updater.enabled') return Promise.resolve(true);
        return Promise.resolve(null);
      }
      return Promise.resolve(null);
    });

    const existing = document.getElementById('modal-root');
    if (existing) existing.remove();
    const root = document.createElement('div');
    root.id = 'modal-root';
    document.body.appendChild(root);
  });

  it('renders Settings header + Appearance + Capture + Clipboard + OCR + Updates + About sections', async () => {
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('Capture')).toBeInTheDocument();
    expect(screen.getByText('Clipboard')).toBeInTheDocument();
    expect(screen.getByText('OCR')).toBeInTheDocument();
    expect(screen.getByText('Updates')).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'About', level: 2 }),
    ).toBeInTheDocument();
  });

  it('lists all 8 theme cards', async () => {
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
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
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
    const cards = await screen.findAllByText('Memphis Machine');
    fireEvent.click(cards[0]!);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'theme',
        value: expect.stringContaining('memphis'),
      });
    });
  });

  it('changing the capture format calls setSetting', async () => {
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
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
      if (cmd === 'plugin:snk-updater|get_update_status')
        return Promise.resolve({ kind: 'idle' });
      if (cmd === 'plugin:snk-updater|get_last_check_at')
        return Promise.resolve(null);
      return Promise.resolve(undefined);
    });
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
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

  it('toggling Hide-own-windows persists the new boolean', async () => {
    mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'plugin:snk-library|get_setting') {
        const key = (args as { key: string }).key;
        if (key === 'capture.hide_own_windows') return Promise.resolve(true);
        return Promise.resolve(null);
      }
      if (cmd === 'plugin:snk-updater|get_update_status')
        return Promise.resolve({ kind: 'idle' });
      if (cmd === 'plugin:snk-updater|get_last_check_at')
        return Promise.resolve(null);
      return Promise.resolve(undefined);
    });
    renderWithQuery(
      <ModalProvider>
        <SettingsWindow />
      </ModalProvider>,
    );
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalled());

    const rowLabel = await screen.findByText(/Hide snapper-keeper/i);
    const row = rowLabel.closest('div.flex')!;
    const toggle = row.querySelector('button');
    expect(toggle).toBeTruthy();

    await act(async () => fireEvent.click(toggle!));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'capture.hide_own_windows',
        value: false,
      });
    });

    it('toggling update checks persists the new boolean', async () => {
      mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
        if (cmd === 'plugin:snk-library|get_setting') {
          const key = (args as { key: string }).key;
          if (key === 'updater.enabled') return Promise.resolve(true);
          return Promise.resolve(null);
        }
        if (cmd === 'plugin:snk-updater|get_update_status')
          return Promise.resolve({ kind: 'idle' });
        if (cmd === 'plugin:snk-updater|get_last_check_at')
          return Promise.resolve(null);
        return Promise.resolve(undefined);
      });
      renderWithQuery(
        <ModalProvider>
          <SettingsWindow />
        </ModalProvider>,
      );
      await waitFor(() => expect(mockedInvoke).toHaveBeenCalled());

      const rowLabel = await screen.findByText(/Enable update checks/i);
      const row = rowLabel.closest('div.flex')!;
      const toggle = row.querySelector('button');
      expect(toggle).toBeTruthy();

      await act(async () => fireEvent.click(toggle!));
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
          key: 'updater.enabled',
          value: false,
        });
      });
    });
  });
});
