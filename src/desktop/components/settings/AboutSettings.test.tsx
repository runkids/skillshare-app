import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import AboutSettings from './AboutSettings';
import { resetAppUpdateForTests } from '../../hooks/useAppUpdate';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.0.12') }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('../../context/TauriContext', () => ({ useTauri: () => ({ appInfo: null }) }));
vi.mock('../../hooks/useUpdates', () => ({ useUpdates: () => ({ cli: null, app: null }) }));
vi.mock('../../api/tauri-bridge', () => ({ tauriBridge: { openLogsFolder: vi.fn() } }));

function availableUpdate() {
  const update = { version: '0.0.13', downloadAndInstall: vi.fn(() => Promise.resolve()) };
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
