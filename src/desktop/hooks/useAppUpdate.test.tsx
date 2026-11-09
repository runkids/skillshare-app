import { render, renderHook, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import SettingsPage from '../pages/SettingsPage';
import { resetAppUpdateForTests, useAppUpdate } from './useAppUpdate';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));

const updates = vi.hoisted(() => ({
  cli: null as string | null,
  app: null as string | null,
  skills: [] as string[],
  agents: [] as string[],
  plugins: [] as string[],
  repositories: [] as string[],
}));
vi.mock('./useUpdates', () => ({ useUpdates: () => updates }));

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
  mockCheck.mockReset();
  updates.cli = null;
  updates.skills = [];
});

describe('app update check', () => {
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
