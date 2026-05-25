import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { ClipboardSettings } from './ClipboardSettings';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<ClipboardSettings />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it('renders empty state when setting is unset', async () => {
    mockedInvoke.mockResolvedValueOnce(null);
    renderWithQuery(<ClipboardSettings />);
    expect(await screen.findByText(/no exclusions configured/i)).toBeInTheDocument();
  });

  it('renders entries from the setting value', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'com.1password.1password8', display_name: '1Password 8', kind: 'macos_bundle_id' },
      { identifier: 'KeePassXC.exe', display_name: 'KeePassXC', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);
    expect(await screen.findByText('1Password 8')).toBeInTheDocument();
    expect(screen.getByText('KeePassXC')).toBeInTheDocument();
  });

  it('Add app modal submits a new entry via set_setting', async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    renderWithQuery(<ClipboardSettings />);

    fireEvent.click(await screen.findByText(/add app/i));
    fireEvent.change(screen.getByPlaceholderText(/com.example.app/i), {
      target: { value: 'com.bitwarden.desktop' },
    });
    mockedInvoke.mockResolvedValueOnce(undefined);
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'com.bitwarden.desktop', display_name: 'com.bitwarden.desktop', kind: 'macos_bundle_id' },
    ]);
    fireEvent.click(screen.getByText(/^add$/i));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'clipboard.app_blocklist',
        value: [
          {
            identifier: 'com.bitwarden.desktop',
            display_name: 'com.bitwarden.desktop',
            kind: 'macos_bundle_id',
          },
        ],
      });
    });
  });

  it('add-from-frontmost calls detect_frontmost_app and prefills the confirmation modal', async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    renderWithQuery(<ClipboardSettings />);

    mockedInvoke.mockResolvedValueOnce({
      identifier: 'com.1password.1password8',
      display_name: '1Password 8',
      kind: 'macos_bundle_id',
    });
    fireEvent.click(await screen.findByText(/add from frontmost/i));

    expect(await screen.findByText(/block frontmost app/i)).toBeInTheDocument();
    expect(screen.getByText('1Password 8')).toBeInTheDocument();
  });

  it('remove button persists the updated list', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'foo.exe', display_name: 'Foo', kind: 'windows_exe' },
      { identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);

    fireEvent.click(await screen.findByLabelText(/remove foo/i));
    mockedInvoke.mockResolvedValueOnce(undefined);
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' },
    ]);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'clipboard.app_blocklist',
        value: [{ identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' }],
      });
    });
  });

  it('duplicate identifier blocks add with inline error', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'foo.exe', display_name: 'Foo', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);
    fireEvent.click(await screen.findByText(/add app/i));

    fireEvent.change(screen.getByRole('combobox'), {
      target: { value: 'windows_exe' },
    });
    fireEvent.change(screen.getByPlaceholderText(/example.exe/i), {
      target: { value: 'foo.exe' },
    });
    fireEvent.click(screen.getByText(/^add$/i));

    expect(await screen.findByText(/already in the list/i)).toBeInTheDocument();
  });
});
