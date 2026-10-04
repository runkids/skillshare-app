import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CliSettings from './CliSettings';

const mocks = vi.hoisted(() => ({
  upgradeCli: vi.fn(),
  runCli: vi.fn(),
  refresh: vi.fn(() => Promise.resolve()),
  reloadView: vi.fn(),
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (e: { payload: unknown }) => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));

vi.mock('../../api/tauri-bridge', () => ({
  CLI_UPGRADE_STEP_EVENT: 'cli-upgrade-step',
  tauriBridge: {
    detectCli: () => Promise.resolve(null),
    upgradeCli: mocks.upgradeCli,
    runCli: mocks.runCli,
  },
}));
vi.mock('../../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { cliVersion: 'v0.23.0' }, refresh: mocks.refresh }),
}));
vi.mock('../../context/ProjectContext', () => ({
  useProjects: () => ({ reloadView: mocks.reloadView }),
}));
vi.mock('../../hooks/useUpdates', () => ({
  useUpdates: () => ({ cli: 'v0.24.0', app: null }),
  checkUpdatesNow: () => Promise.resolve(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => vi.useRealTimers());

describe('CliSettings upgrade', () => {
  it('upgrades through the app so the server restarts, not by running the CLI directly', async () => {
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    render(<CliSettings />);
    fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI' }));
    expect(await screen.findByText('Updated to v0.24.0')).toBeInTheDocument();
    expect(mocks.runCli).not.toHaveBeenCalled();
  });

  it('reloads the web view after the restarted server is up', async () => {
    mocks.upgradeCli.mockResolvedValue('v0.24.0');
    render(<CliSettings />);
    fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI' }));
    await screen.findByText('Updated to v0.24.0');
    expect(mocks.reloadView).toHaveBeenCalledTimes(1);
  });

  it('offers a retry when the upgrade fails', async () => {
    mocks.upgradeCli.mockRejectedValue('CLI exited with 1: network error');
    render(<CliSettings />);
    fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI' }));
    expect(await screen.findByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('shows the step a slow upgrade is on and how long it has run', async () => {
    vi.useFakeTimers();
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    render(<CliSettings />);
    fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI' }));
    await act(() => vi.advanceTimersByTimeAsync(83_000));
    act(() => mocks.handlers.get('cli-upgrade-step')?.({ payload: 'Downloading v0.24.1...' }));
    expect(screen.getByText('Downloading v0.24.1... 1:23')).toBeInTheDocument();
  });
});
