import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import {
  checkVersionsNow,
  resetUpdatesForTests,
  useUpdates,
  useVersionResults,
} from './useUpdates';
import { refreshUpdatePolicy, resetUpdatePolicyForTests } from './useUpdatePolicy';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
  check: vi.fn(),
  policyEnabled: false,
}));
const empty = { cli: null, app: null, skills: [], repositories: [], agents: [], plugins: [] };
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (name: string, handler: (e: { payload: unknown }) => void) => {
    mocks.handlers.set(name, handler);
    return Promise.resolve(() => {});
  },
}));
vi.mock('../api/tauri-bridge', () => ({
  tauriBridge: {
    getUpdatePolicy: () =>
      Promise.resolve({
        enabled: mocks.policyEnabled,
        configured: false,
        envOverride: false,
        supported: true,
      }),
    getAvailableUpdates: () =>
      Promise.resolve({
        cli: '9.9.9',
        app: '9.9.9',
        skills: [],
        repositories: [],
        agents: [],
        plugins: [],
      }),
    checkVersionsNow: mocks.check,
  },
}));

beforeEach(() => {
  mocks.policyEnabled = false;
  resetUpdatesForTests();
  resetUpdatePolicyForTests();
  mocks.handlers.clear();
  mocks.check.mockReset();
});

it('keeps masked snapshots stable for status consumers', async () => {
  const { result, rerender } = renderHook(() => useUpdates());
  await waitFor(() => expect(mocks.handlers.has('updates-available')).toBe(true));
  await act(async () => {});
  const previous = result.current;
  rerender();
  expect(result.current).toBe(previous);
  expect(result.current.cli).toBeNull();
});

it('retains manual results after disabled-policy background clearing without restoring badges', async () => {
  const { result } = renderHook(() => ({
    notifications: useUpdates(),
    manual: useVersionResults(),
  }));
  await act(async () => {});
  mocks.check.mockResolvedValue({ ...empty, cli: '9.9.9' });
  await act(async () => {
    await checkVersionsNow();
  });
  act(() => mocks.handlers.get('updates-available')?.({ payload: empty }));
  expect(result.current.manual.cli).toBe('9.9.9');
  expect(result.current.notifications.cli).toBeNull();
});

it('retains manual results when opt-out is published before cache clearing', async () => {
  mocks.policyEnabled = true;
  const { result } = renderHook(() => ({
    notifications: useUpdates(),
    manual: useVersionResults(),
  }));
  await act(async () => {});
  mocks.check.mockResolvedValue({ ...empty, cli: '9.9.9' });
  await act(async () => {
    await checkVersionsNow();
  });
  act(() => {
    mocks.handlers.get('update-policy-changed')?.({
      payload: { enabled: false, configured: false, envOverride: false, supported: true },
    });
    mocks.handlers.get('updates-available')?.({ payload: empty });
  });
  expect(result.current.manual.cli).toBe('9.9.9');
  expect(result.current.notifications.cli).toBeNull();
});

it('lets an enabled automatic check replace older manual results', async () => {
  const { result } = renderHook(() => useVersionResults());
  await act(async () => {});
  mocks.check.mockResolvedValueOnce(empty);
  await act(async () => {
    await checkVersionsNow();
  });
  mocks.policyEnabled = true;
  await act(async () => {
    await refreshUpdatePolicy();
  });
  mocks.check.mockResolvedValueOnce({ ...empty, cli: '10.0.0' });
  await act(async () => {
    await checkVersionsNow(true);
  });
  expect(result.current.cli).toBe('10.0.0');
});

it('queues an explicit check when an automatic request is already in flight', async () => {
  let finish!: (value: typeof empty) => void;
  mocks.check.mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve;
    })
  );
  mocks.check.mockResolvedValueOnce({ ...empty, cli: '9.9.9' });
  const automatic = checkVersionsNow(true);
  const manual = checkVersionsNow();
  finish(empty);
  await Promise.all([automatic, manual]);
  expect(mocks.check.mock.calls.map((args) => args[0])).toEqual([true, false]);
});
