import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CliSettings from './CliSettings';

const mocks = vi.hoisted(() => ({
  upgradeCli: vi.fn(),
  runCli: vi.fn(),
  refresh: vi.fn(() => Promise.resolve()),
  reloadView: vi.fn(),
}));

vi.mock('../../api/tauri-bridge', () => ({
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

  it('shows how long a slow upgrade has been running', async () => {
    vi.useFakeTimers();
    mocks.upgradeCli.mockReturnValue(new Promise(() => {}));
    render(<CliSettings />);
    fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI' }));
    await act(() => vi.advanceTimersByTimeAsync(83_000));
    expect(screen.getByText(/Downloading and installing… 1:23/)).toBeInTheDocument();
  });
});
