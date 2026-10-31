import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import UpdatesSettings from './UpdatesSettings';
import { JUST_UPDATED_KEY, resetAppUpdateForTests } from '../../hooks/useAppUpdate';
import { resetCliUpgradeForTests } from '../../hooks/useCliUpgrade';

const mocks = vi.hoisted(() => ({
  upgradeCli: vi.fn(),
  runCli: vi.fn(),
  refresh: vi.fn(() => Promise.resolve()),
  reloadView: vi.fn(),
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
  updates: {
    cli: null as string | null,
    app: null as string | null,
    skills: [] as string[],
    agents: [] as string[],
    plugins: [] as string[],
    repositories: [] as string[],
  },
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.7.6') }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (e: { payload: unknown }) => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../../api/tauri-bridge', () => ({
  CLI_UPGRADE_STEP_EVENT: 'cli-upgrade-step',
  tauriBridge: {
    upgradeCli: mocks.upgradeCli,
    runCli: mocks.runCli,
    getNotifyUpdate: () => Promise.resolve(true),
    setNotifyUpdate: () => Promise.resolve(),
  },
}));
vi.mock('../../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { cliVersion: 'v0.23.0' }, refresh: mocks.refresh }),
}));
vi.mock('../../context/ProjectContext', () => ({
  useProjects: () => ({ reloadView: mocks.reloadView }),
}));
vi.mock('../../hooks/useUpdates', () => ({
  useUpdates: () => mocks.updates,
  checkUpdatesNow: () => Promise.resolve(),
}));

function appUpdate(body = '') {
  const update = { version: '0.7.7', body, downloadAndInstall: vi.fn(() => Promise.resolve()) };
  vi.mocked(check).mockResolvedValue(update as unknown as Awaited<ReturnType<typeof check>>);
  return update;
}

function renderUpdates() {
  return render(
    <MemoryRouter>
      <UpdatesSettings />
    </MemoryRouter>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetAppUpdateForTests();
  resetCliUpgradeForTests();
  vi.mocked(check).mockResolvedValue(null);
  mocks.updates.cli = null;
  localStorage.clear();
});
afterEach(() => vi.useRealTimers());

describe('UpdatesSettings CLI', () => {
  it('upgrades through the app so the server restarts, then reloads the web view', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Update CLI' }));
    expect(await screen.findByText('CLI updated to v0.24.0')).toBeInTheDocument();
    expect([mocks.runCli.mock.calls.length, mocks.reloadView.mock.calls.length]).toEqual([0, 1]);
  });

  it('offers a retry when the upgrade fails', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockRejectedValue('CLI exited with 1: network error');
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Update CLI' }));
    expect(await screen.findByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('shows the step a slow upgrade is on and how long it has run', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Update CLI' }));
    await act(() => vi.advanceTimersByTimeAsync(83_000));
    act(() => mocks.handlers.get('cli-upgrade-step')?.({ payload: 'Downloading v0.24.1...' }));
    expect(screen.getByText('Downloading v0.24.1... 1:23')).toBeInTheDocument();
  });

  it('shows an indeterminate bar while the CLI updates, since it reports no percentage', () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Update CLI' }));
    expect(screen.getByRole('progressbar', { name: 'CLI update' })).not.toHaveAttribute(
      'aria-valuenow'
    );
  });
});

describe('UpdatesSettings app', () => {
  it('restarts on its own after installing and remembers what changed', async () => {
    appUpdate('* faster sync');
    vi.mocked(relaunch).mockResolvedValue(undefined);
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    await waitFor(() => expect(relaunch).toHaveBeenCalledTimes(1));
    expect(JSON.parse(localStorage.getItem(JUST_UPDATED_KEY) ?? '{}')).toEqual({
      version: '0.7.7',
      notes: '* faster sync',
    });
  });

  it('offers a manual restart when the automatic one fails', async () => {
    appUpdate();
    vi.mocked(relaunch).mockRejectedValue(new Error('denied'));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    expect(await screen.findByRole('button', { name: 'Restart now' })).toBeInTheDocument();
  });

  it('shows release notes as plain text', async () => {
    appUpdate('* <b>Faster sync</b>');
    renderUpdates();
    expect(await screen.findByText('<b>Faster sync</b>')).toBeInTheDocument();
  });

  it('updates the CLI before the app when both are new', async () => {
    const update = appUpdate();
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    vi.mocked(relaunch).mockResolvedValue(undefined);
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    await waitFor(() => expect(relaunch).toHaveBeenCalledTimes(1));
    expect(mocks.upgradeCli.mock.invocationCallOrder[0]).toBeLessThan(
      update.downloadAndInstall.mock.invocationCallOrder[0]
    );
  });
});
