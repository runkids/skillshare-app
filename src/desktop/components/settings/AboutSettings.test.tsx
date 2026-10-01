import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { openUrl } from '@tauri-apps/plugin-opener';
import AboutSettings from './AboutSettings';
import { resetAppUpdateForTests } from '../../hooks/useAppUpdate';
import { tauriBridge } from '../../api/tauri-bridge';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.0.12') }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../../context/TauriContext', () => ({ useTauri: () => ({ appInfo: null }) }));
vi.mock('../../hooks/useUpdates', () => ({ useUpdates: () => ({ cli: null, app: null }) }));
vi.mock('../../api/tauri-bridge', () => ({
  tauriBridge: { openLogsFolder: vi.fn(), exportDiagnostics: vi.fn() },
}));

function availableUpdate(body?: string) {
  const update = { version: '0.0.13', body, downloadAndInstall: vi.fn(() => Promise.resolve()) };
  vi.mocked(check).mockResolvedValue(update as unknown as Awaited<ReturnType<typeof check>>);
  return update;
}

function renderAbout() {
  return render(
    <MemoryRouter>
      <AboutSettings />
    </MemoryRouter>
  );
}

beforeEach(() => {
  resetAppUpdateForTests();
  vi.mocked(relaunch).mockReset();
});

describe('AboutSettings app update', () => {
  it('restarts the app on its own once the update is installed', async () => {
    availableUpdate();
    vi.mocked(relaunch).mockResolvedValue(undefined);
    renderAbout();
    fireEvent.click(await screen.findByRole('button', { name: 'Update Now' }));
    await waitFor(() => expect(relaunch).toHaveBeenCalledTimes(1));
  });

  it('offers a manual restart when the automatic one fails', async () => {
    availableUpdate();
    vi.mocked(relaunch).mockRejectedValue(new Error('denied'));
    renderAbout();
    fireEvent.click(await screen.findByRole('button', { name: 'Update Now' }));
    expect(await screen.findByRole('button', { name: 'Restart' })).toBeInTheDocument();
  });
});

describe('AboutSettings links', () => {
  it('opens the release notes for the running version in the system browser', async () => {
    renderAbout();
    fireEvent.click(await screen.findByRole('link', { name: 'v0.0.12' }));
    expect(openUrl).toHaveBeenCalledWith(
      'https://github.com/runkids/skillshare-app/releases/tag/v0.0.12'
    );
  });

  it('points the GitHub row at the app repository', () => {
    renderAbout();
    expect(screen.getByRole('link', { name: 'runkids/skillshare-app' })).toHaveAttribute(
      'href',
      'https://github.com/runkids/skillshare-app'
    );
  });

  it('shows the notes of an available update as plain text', async () => {
    availableUpdate('<b>Faster sync</b>');
    renderAbout();
    expect(await screen.findByText('<b>Faster sync</b>')).toBeInTheDocument();
  });
});

describe('AboutSettings diagnostics', () => {
  it('reports where the diagnostics file was saved', async () => {
    vi.mocked(tauriBridge.exportDiagnostics).mockResolvedValue('~/Downloads/diag.txt');
    renderAbout();
    fireEvent.click(screen.getByRole('button', { name: 'Export diagnostics' }));
    expect(await screen.findByText('Saved to ~/Downloads/diag.txt')).toBeInTheDocument();
  });

  it('shows the error when the export fails', async () => {
    vi.mocked(tauriBridge.exportDiagnostics).mockRejectedValue('Disk full');
    renderAbout();
    fireEvent.click(screen.getByRole('button', { name: 'Export diagnostics' }));
    expect(await screen.findByText('Disk full')).toBeInTheDocument();
  });
});
