import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';

import { ModalProvider } from '../../components/Modal';
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

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedListen.mockReset().mockResolvedValue(() => {});
  vi.mocked(getVersion).mockClear();

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
  } = {},
) {
  mockedInvoke.mockImplementation((cmd: string) => {
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
      expect(openUrl).toHaveBeenCalledWith(expect.stringContaining('github.com'));
    });
  });

  it('clicking Open on a path row calls openPath', async () => {
    setStatusResponses();
    const { openPath } = await import('@tauri-apps/plugin-opener');
    renderAbout();
    // Wait for the data-dir path to render — that signals useQuery has
    // resolved and the Open button is no longer disabled. Without this,
    // CI's slower scheduler clicks the button while dataDirQ.data is
    // still undefined, the onClick short-circuits, and openPath is
    // never called.
    await screen.findByText('/mock/data');
    const openButtons = screen.getAllByRole('button', { name: /Open/i });
    expect(openButtons.length).toBeGreaterThanOrEqual(2);
    fireEvent.click(openButtons[0]!);
    await waitFor(() => {
      expect(openPath).toHaveBeenCalled();
    });
  });
});
