import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import GeneralSettings from './GeneralSettings';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: boolean }) => void>(),
  off: vi.fn(),
  getNotifySync: vi.fn(() => Promise.resolve(true)),
  setNotifySync: vi.fn<(enabled: boolean) => Promise<void>>(() => Promise.resolve()),
  setAutoSync: vi.fn<(enabled: boolean) => Promise<void>>(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (event: { payload: boolean }) => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(mocks.off);
  },
}));

vi.mock('../../api/tauri-bridge', () => ({
  AUTO_SYNC_CHANGED_EVENT: 'auto-sync-changed',
  tauriBridge: {
    getPreferredPort: () => Promise.resolve(19420),
    getNotifyUpdate: () => Promise.resolve(true),
    getNotifySync: mocks.getNotifySync,
    setNotifySync: mocks.setNotifySync,
    getAutoSync: () => Promise.resolve(false),
    setAutoSync: mocks.setAutoSync,
  },
}));
vi.mock('../../context/TauriContext', () => ({ useTauri: () => ({ refresh: vi.fn() }) }));
vi.mock('../../context/ProjectContext', () => ({ useProjects: () => ({ refresh: vi.fn() }) }));

function syncSwitch() {
  return screen.getByRole('switch', { name: 'Sync notifications' });
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.handlers.clear();
});

describe('GeneralSettings sync notifications', () => {
  it('shows the saved setting and persists a toggle', async () => {
    mocks.getNotifySync.mockResolvedValueOnce(false);
    render(
      <MemoryRouter>
        <GeneralSettings />
      </MemoryRouter>
    );
    await waitFor(() => expect(syncSwitch().getAttribute('aria-checked')).toBe('false'));
    fireEvent.click(syncSwitch());
    await waitFor(() => expect(mocks.setNotifySync).toHaveBeenCalledWith(true));
    expect(syncSwitch().getAttribute('aria-checked')).toBe('true');
  });

  it('rolls the switch back when saving fails', async () => {
    mocks.setNotifySync.mockRejectedValueOnce(new Error('disk full'));
    render(
      <MemoryRouter>
        <GeneralSettings />
      </MemoryRouter>
    );
    await waitFor(() => expect(mocks.getNotifySync).toHaveBeenCalled());
    fireEvent.click(syncSwitch());
    await screen.findByText('disk full');
    expect(syncSwitch().getAttribute('aria-checked')).toBe('true');
  });
});

describe('GeneralSettings auto-sync', () => {
  it('follows tray changes without saving them again and cleans up its subscription', async () => {
    const { unmount } = render(
      <MemoryRouter>
        <GeneralSettings />
      </MemoryRouter>
    );
    await act(async () => {});
    act(() => mocks.handlers.get('auto-sync-changed')?.({ payload: true }));
    expect(screen.getByRole('switch', { name: 'Auto-sync' }).getAttribute('aria-checked')).toBe(
      'true'
    );
    expect(mocks.setAutoSync).not.toHaveBeenCalled();
    act(() => mocks.handlers.get('auto-sync-changed')?.({ payload: false }));
    expect(screen.getByRole('switch', { name: 'Auto-sync' }).getAttribute('aria-checked')).toBe(
      'false'
    );
    unmount();
    await act(async () => {});
    expect(mocks.off).toHaveBeenCalled();
  });

  it('starts off and persists turning it on', async () => {
    render(
      <MemoryRouter>
        <GeneralSettings />
      </MemoryRouter>
    );
    const autoSync = screen.getByRole('switch', { name: 'Auto-sync' });
    expect(autoSync.getAttribute('aria-checked')).toBe('false');
    fireEvent.click(autoSync);
    await waitFor(() => expect(mocks.setAutoSync).toHaveBeenCalledWith(true));
  });
});
