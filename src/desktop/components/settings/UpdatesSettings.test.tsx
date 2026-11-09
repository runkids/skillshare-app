import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import UpdatesSettings from './UpdatesSettings';
import { JUST_UPDATED_KEY, resetAppUpdateForTests } from '../../hooks/useAppUpdate';
import { resetCliUpgradeForTests } from '../../hooks/useCliUpgrade';

const mocks = vi.hoisted(() => ({
  upgradeCli: vi.fn(),
  relaunch: vi.fn(),
  runCli: vi.fn(),
  refresh: vi.fn(() => Promise.resolve()),
  reloadView: vi.fn(),
  checkVersionsNow: vi.fn(() => Promise.resolve()),
  checkResourcesNow: vi.fn(() => Promise.resolve()),
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
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../../api/tauri-bridge', () => ({
  CLI_UPGRADE_STEP_EVENT: 'cli-upgrade-step',
  tauriBridge: {
    upgradeCli: mocks.upgradeCli,
    relaunchApp: mocks.relaunch,
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
  checkVersionsNow: mocks.checkVersionsNow,
  checkResourcesNow: mocks.checkResourcesNow,
}));

type Progress = (e: { event: string; data: Record<string, number> }) => void;

function appUpdate(
  body = '',
  download: (report: Progress) => Promise<void> = () => Promise.resolve()
) {
  const update = { version: '0.7.7', body, downloadAndInstall: vi.fn(download) };
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
    fireEvent.click(await screen.findByRole('button', { name: 'Update CLI' }));
    expect(await screen.findByText('CLI updated')).toBeInTheDocument();
    expect([mocks.runCli.mock.calls.length, mocks.reloadView.mock.calls.length]).toEqual([0, 1]);
  });

  it('offers a retry when the upgrade fails', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockRejectedValue('CLI exited with 1: network error');
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update CLI' }));
    expect(await screen.findByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('shows the step a slow upgrade is on and how long it has run', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    await act(() => vi.advanceTimersByTimeAsync(0));
    fireEvent.click(screen.getByRole('button', { name: 'Update CLI' }));
    await act(() => vi.advanceTimersByTimeAsync(83_000));
    act(() => mocks.handlers.get('cli-upgrade-step')?.({ payload: 'Downloading v0.24.1...' }));
    expect(screen.getByText('Downloading v0.24.1... 1:23')).toBeInTheDocument();
  });

  it('keeps the bar still while the CLI updates, since it reports no percentage', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update CLI' }));
    expect(screen.getByRole('progressbar', { name: 'Update progress' })).toHaveAttribute(
      'aria-valuenow',
      '0'
    );
  });
});

describe('UpdatesSettings app', () => {
  it('restarts on its own after installing and remembers what changed', async () => {
    appUpdate('* faster sync');
    mocks.relaunch.mockResolvedValue(undefined);
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    await waitFor(() => expect(mocks.relaunch).toHaveBeenCalledTimes(1));
    expect(JSON.parse(localStorage.getItem(JUST_UPDATED_KEY) ?? '{}')).toEqual({
      version: '0.7.7',
      notes: '* faster sync',
    });
  });

  it('offers a manual restart when the automatic one fails', async () => {
    appUpdate();
    mocks.relaunch.mockRejectedValue(new Error('denied'));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    expect(await screen.findByRole('button', { name: 'Restart now' })).toBeInTheDocument();
  });

  it('updates the CLI before the app when both are new', async () => {
    const update = appUpdate();
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    mocks.relaunch.mockResolvedValue(undefined);
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    await waitFor(() => expect(mocks.relaunch).toHaveBeenCalledTimes(1));
    expect(mocks.upgradeCli.mock.invocationCallOrder[0]).toBeLessThan(
      update.downloadAndInstall.mock.invocationCallOrder[0]
    );
  });

  it('lists every step of Update both while the CLI still runs', async () => {
    appUpdate();
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    expect(screen.getAllByRole('listitem')).toHaveLength(4);
  });

  it('hides the row buttons while an update runs', async () => {
    appUpdate();
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    expect(screen.queryByRole('button', { name: 'Update and restart' })).not.toBeInTheDocument();
  });

  it('moves the bar by finished steps plus the download share', async () => {
    appUpdate('', (report) => {
      report({ event: 'Started', data: { contentLength: 100 } });
      report({ event: 'Progress', data: { chunkLength: 50 } });
      return new Promise(() => {});
    });
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    await waitFor(() =>
      expect(screen.getByRole('progressbar', { name: 'Update progress' })).toHaveAttribute(
        'aria-valuenow',
        '38'
      )
    );
  });
});

describe('UpdatesSettings checks', () => {
  it('checks the app and CLI versions on entry', async () => {
    renderUpdates();
    await waitFor(() =>
      expect([
        mocks.checkVersionsNow.mock.calls.length,
        vi.mocked(check).mock.calls.length,
      ]).toEqual([1, 1])
    );
  });

  it('leaves skills, agents and plugins out of the versions check', async () => {
    renderUpdates();
    const button = screen.getByRole('button', { name: 'Check versions' });
    await waitFor(() => expect(button).toBeEnabled());
    fireEvent.click(button);
    await waitFor(() => expect(mocks.checkVersionsNow).toHaveBeenCalledTimes(2));
    expect(mocks.checkResourcesNow).not.toHaveBeenCalled();
  });

  it('checks skills, agents and plugins from their own section', () => {
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Check skills, agents and plugins' }));
    expect(mocks.checkResourcesNow).toHaveBeenCalledTimes(1);
  });

  it('shows how long the skills check has run', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    mocks.checkResourcesNow.mockReturnValueOnce(new Promise(() => {}));
    renderUpdates();
    fireEvent.click(screen.getByRole('button', { name: 'Check skills, agents and plugins' }));
    await act(() => vi.advanceTimersByTimeAsync(8_000));
    expect(screen.getByText('Checking every source... 0:08')).toBeInTheDocument();
  });

  it('says the app version could not be checked instead of up to date', async () => {
    vi.mocked(check).mockRejectedValue(new Error('network down'));
    renderUpdates();
    expect(await screen.findByText('Couldn’t check')).toBeInTheDocument();
  });

  it('keeps the versions check usable while the app waits for a manual restart', async () => {
    appUpdate();
    mocks.relaunch.mockRejectedValue(new Error('denied'));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    await screen.findByRole('button', { name: 'Restart now' });
    expect(screen.getByRole('button', { name: 'Check versions' })).toBeEnabled();
  });

  it('keeps every step of the run when checking while the app waits for a restart', async () => {
    appUpdate();
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    mocks.relaunch.mockRejectedValue(new Error('denied'));
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update both' }));
    await screen.findByRole('button', { name: 'Restart now' });
    fireEvent.click(screen.getByRole('button', { name: 'Check versions' }));
    expect(screen.getAllByRole('listitem')).toHaveLength(4);
  });

  it('clears a finished CLI update when checking again', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update CLI' }));
    await screen.findByText('CLI updated');
    fireEvent.click(screen.getByRole('button', { name: 'Check versions' }));
    expect(screen.queryByText('CLI updated')).not.toBeInTheDocument();
  });
});

describe('UpdatesSettings on leaving', () => {
  it('clears a finished CLI update', async () => {
    mocks.updates.cli = 'v0.24.0';
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    const { unmount } = renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update CLI' }));
    await screen.findByText('CLI updated');
    unmount();
    renderUpdates();
    expect(screen.queryByText('CLI updated')).not.toBeInTheDocument();
  });

  it('clears an app update that waits for a manual restart', async () => {
    appUpdate();
    mocks.relaunch.mockRejectedValue(new Error('denied'));
    const { unmount } = renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    await screen.findByRole('button', { name: 'Restart now' });
    unmount();
    renderUpdates();
    expect(screen.queryByRole('button', { name: 'Restart now' })).not.toBeInTheDocument();
  });

  it('keeps an app update that is still downloading', async () => {
    appUpdate('', () => new Promise(() => {}));
    const { unmount } = renderUpdates();
    fireEvent.click(await screen.findByRole('button', { name: 'Update and restart' }));
    await screen.findByText('Download skillshare App v0.7.7');
    unmount();
    renderUpdates();
    expect(screen.getByText('Download skillshare App v0.7.7')).toBeInTheDocument();
  });
});
