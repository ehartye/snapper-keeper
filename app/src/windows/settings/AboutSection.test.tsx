import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import { openPath } from '@tauri-apps/plugin-opener';

import { ModalProvider } from '../../components/Modal';
import { isStoreEdition } from '../../lib/storeEdition';
import { AboutSection } from './AboutSection';
import { renderWithQuery } from '../../test/renderWithQuery';

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.1.2'),
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn().mockResolvedValue('/mock/data'),
  appLogDir: vi.fn().mockResolvedValue('/mock/log'),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn().mockResolvedValue(undefined),
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../../lib/storeEdition', () => ({
  isStoreEdition: vi.fn(() => false),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);
const mockedIsStoreEdition = vi.mocked(isStoreEdition);

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedListen.mockReset().mockResolvedValue(() => {});
  mockedIsStoreEdition.mockReturnValue(false);
  vi.mocked(getVersion).mockClear();
  vi.mocked(openPath).mockReset().mockResolvedValue(undefined);

  const existing = document.getElementById('modal-root');
  if (existing) existing.remove();
  const root = document.createElement('div');
  root.id = 'modal-root';
  document.body.appendChild(root);
});

function setStatusResponses(
  opts: {
    lastCheckedAt?: number | null;
    status?: { kind: string; [k: string]: unknown };
    updaterEnabled?: boolean;
  } = {},
) {
  mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
    if (cmd === 'plugin:snk-updater|get_update_status') {
      return Promise.resolve(opts.status ?? { kind: 'idle' });
    }
    if (cmd === 'plugin:snk-updater|get_last_check_at') {
      return Promise.resolve(opts.lastCheckedAt ?? null);
    }
    if (cmd === 'plugin:snk-updater|check_for_update') {
      return Promise.resolve(opts.status ?? { kind: 'idle' });
    }
    if (cmd === 'plugin:snk-updater|restart_app') {
      return Promise.resolve(undefined);
    }
    if (cmd === 'plugin:snk-library|get_setting') {
      const key = (args as { key: string }).key;
      if (key === 'updater.enabled') {
        return Promise.resolve(opts.updaterEnabled ?? true);
      }
      return Promise.resolve(null);
    }
    return Promise.resolve(null);
  });
}

function renderAbout() {
  return renderWithQuery(
    <ModalProvider>
      <AboutSection />
    </ModalProvider>,
  );
}

describe('<AboutSection />', () => {
  it('renders the section header', async () => {
    setStatusResponses();
    renderAbout();
    expect(
      await screen.findByRole('heading', { name: 'About', level: 2 }),
    ).toBeInTheDocument();
  });

  it('renders the app version with git sha', async () => {
    setStatusResponses();
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText(/0\.1\.2 \(.+\)/)).toBeInTheDocument();
    });
  });

  it('renders the data dir and log dir paths', async () => {
    setStatusResponses();
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText('/mock/data')).toBeInTheDocument();
      expect(screen.getByText('/mock/log')).toBeInTheDocument();
    });
  });

  it('renders the updater fingerprint', async () => {
    setStatusResponses();
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText('testfingerprint')).toBeInTheDocument();
    });
  });

  it('renders "never" for last check when null', async () => {
    setStatusResponses({ lastCheckedAt: null });
    renderAbout();
    await waitFor(() => {
      const row = screen.getByText(/Last check/i).closest('div.flex')!;
      expect(row.textContent).toMatch(/never/i);
    });
  });

  it('renders the updater status text for idle', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByText(/Up to date/i)).toBeInTheDocument();
    });
  });

  it('renders updater error status with retry hint', async () => {
    setStatusResponses({
      status: { kind: 'error', reason: 'check-failed', retryable: true },
    });
    renderAbout();
    await waitFor(() => {
      expect(
        screen.getByText(/Error \(check-failed\), retry scheduled/i),
      ).toBeInTheDocument();
    });
  });

  it('renders Check Now button which is enabled when idle', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    const btn = await screen.findByRole('button', { name: /Check Now/i });
    expect(btn).toBeEnabled();
  });

  it('Check Now button is disabled while status is checking', async () => {
    setStatusResponses({ status: { kind: 'checking' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Check/i })).toBeDisabled();
    });
  });

  it('renders policy suppression text when update checks are disabled in settings', async () => {
    setStatusResponses({ updaterEnabled: false });
    renderAbout();
    await waitFor(() => {
      expect(
        screen.getByText(/Update checks disabled in Settings/i),
      ).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Check Now/i })).toBeDisabled();
    });
  });

  it('renders Microsoft Store suppression text when updater is compiled out', async () => {
    mockedIsStoreEdition.mockReturnValue(true);
    setStatusResponses();
    renderAbout();
    await waitFor(() => {
      expect(
        screen.getByText(/Updates are managed by Microsoft Store/i),
      ).toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: /Managed by Store/i }),
      ).toBeDisabled();
    });
  });

  it('clicking Check Now calls check_for_update', async () => {
    setStatusResponses({ status: { kind: 'idle' } });
    renderAbout();
    const btn = await screen.findByRole('button', { name: /Check Now/i });
    fireEvent.click(btn);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-updater|check_for_update',
      );
    });
  });

  it('shows restart modal when status reaches "ready"', async () => {
    setStatusResponses({ status: { kind: 'ready', version: '1.2.3' } });
    renderAbout();
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: 'Restart' }),
      ).toBeInTheDocument();
    });
  });

  it('clicking Privacy link calls openUrl with the privacy URL', async () => {
    setStatusResponses();
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    const link = await screen.findByRole('button', { name: /Privacy/i });
    fireEvent.click(link);
    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(expect.stringContaining('github.com'));
    });
  });

  it('clicking License link calls openUrl with the license URL', async () => {
    setStatusResponses();
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    const link = await screen.findByRole('button', { name: /License/i });
    fireEvent.click(link);
    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(
        expect.stringContaining('/LICENSE.md'),
      );
    });
  });

  async function clickOpen(path: string) {
    const pathElement = await screen.findByText(path);
    const row = pathElement.closest('div.flex')!;
    await act(async () => {
      fireEvent.click(within(row).getByRole('button', { name: 'Open' }));
    });
  }

  it('opens the data directory', async () => {
    setStatusResponses();
    renderAbout();
    await clickOpen('/mock/data');
    await waitFor(() => {
      expect(openPath).toHaveBeenCalledWith('/mock/data');
    });
  });

  it('opens the log directory', async () => {
    setStatusResponses();
    renderAbout();
    await clickOpen('/mock/log');
    await waitFor(() => {
      expect(openPath).toHaveBeenCalledWith('/mock/log');
    });
  });

  it('shows an alert when the data directory cannot be opened', async () => {
    setStatusResponses();
    vi.mocked(openPath).mockRejectedValueOnce(new Error('denied'));
    renderAbout();
    await clickOpen('/mock/data');
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not open the data directory.',
    );
  });

  it('shows an alert when the log directory cannot be opened', async () => {
    setStatusResponses();
    vi.mocked(openPath).mockRejectedValueOnce(new Error('denied'));
    renderAbout();
    await clickOpen('/mock/log');
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not open the log directory.',
    );
  });

  it('shows no alert when a directory opens', async () => {
    setStatusResponses();
    vi.mocked(openPath).mockRejectedValueOnce(new Error('denied'));
    renderAbout();
    await clickOpen('/mock/data');
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not open the data directory.',
    );
    await clickOpen('/mock/log');
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });
});
