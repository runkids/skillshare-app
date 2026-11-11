import { act, render, renderHook, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import SettingsPage from '../pages/SettingsPage';
import { resetAppUpdateForTests, useAppUpdate } from './useAppUpdate';
import { refreshUpdatePolicy, resetUpdatePolicyForTests } from './useUpdatePolicy';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: () => Promise.resolve(() => {}) }));

const updates = vi.hoisted(() => ({
  cli: null as string | null,
  app: null as string | null,
  skills: [] as string[],
  agents: [] as string[],
  plugins: [] as string[],
  repositories: [] as string[],
}));
vi.mock('./useUpdates', () => ({ useUpdates: () => updates }));

const policy = vi.hoisted(() => ({
  enabled: true,
  configured: true,
  envOverride: false,
  supported: true,
}));
vi.mock('../api/tauri-bridge', () => ({
  tauriBridge: { getUpdatePolicy: () => Promise.resolve(policy) },
}));
const mockCheck = vi.mocked(check);

function renderSettings() {
  return render(
    <MemoryRouter initialEntries={['/settings?tab=appearance']}>
      <SettingsPage />
    </MemoryRouter>
  );
}

beforeEach(() => {
  resetAppUpdateForTests();
  resetUpdatePolicyForTests();
  policy.enabled = true;
  mockCheck.mockReset();
  updates.cli = null;
  updates.skills = [];
});

describe('app update check', () => {
  it('preserves explicit intent when joining an automatic check after opt-out', async () => {
    let finish!: (value: Awaited<ReturnType<typeof check>>) => void;
    mockCheck.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      })
    );
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(mockCheck).toHaveBeenCalledTimes(1));
    policy.enabled = false;
    await act(async () => {
      await refreshUpdatePolicy();
    });
    await act(async () => {
      const manual = result.current.recheck();
      finish({ version: '9.9.9' } as Awaited<ReturnType<typeof check>>);
      await manual;
    });
    expect(result.current.version).toBe('9.9.9');
    expect(result.current.available).toBe(false);
  });

  it('does not contact the updater when automatic checks are disabled', async () => {
    policy.enabled = false;
    mockCheck.mockResolvedValue(null);
    renderHook(() => useAppUpdate());
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(mockCheck).not.toHaveBeenCalled();
  });

  it('checks only once per session across remounts', async () => {
    mockCheck.mockResolvedValue(null);
    renderHook(() => useAppUpdate()).unmount();
    renderHook(() => useAppUpdate());
    await waitFor(() => expect(mockCheck).toHaveBeenCalledTimes(1));
  });
});

describe('settings nav', () => {
  it('shows no update count on the Updates tab', () => {
    updates.cli = 'v0.24.0';
    updates.skills = ['pdf', 'docx'];
    renderSettings();
    expect(screen.getByRole('button', { name: 'Updates' })).toHaveTextContent(/^Updates$/);
  });
});
