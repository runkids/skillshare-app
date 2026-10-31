import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
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

function renderAbout() {
  return render(
    <MemoryRouter>
      <AboutSettings />
    </MemoryRouter>
  );
}

beforeEach(() => {
  resetAppUpdateForTests();
  vi.mocked(check).mockResolvedValue(null);
});

describe('AboutSettings links', () => {
  it('opens the release notes for the running version in the system browser', async () => {
    renderAbout();
    fireEvent.click(await screen.findByRole('link', { name: /Release notes\s*v0\.0\.12/ }));
    expect(openUrl).toHaveBeenCalledWith(
      'https://github.com/runkids/skillshare-app/releases/tag/v0.0.12'
    );
  });

  it('points the GitHub row at the app repository', () => {
    renderAbout();
    expect(screen.getByRole('link', { name: /App repository/ })).toHaveAttribute(
      'href',
      'https://github.com/runkids/skillshare-app'
    );
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
