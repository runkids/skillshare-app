import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import SettingsPage from '../pages/SettingsPage';
import { resetAppUpdateForTests } from './useAppUpdate';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));

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
});

describe('settings nav update badge', () => {
  it('stays hidden when no update is available', async () => {
    mockCheck.mockResolvedValue(null);
    renderSettings();
    await waitFor(() => expect(mockCheck).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('Update')).not.toBeInTheDocument();
  });

  it('appears with an accessible name when an update is available', async () => {
    mockCheck.mockResolvedValue({ version: '9.9.9' } as Awaited<ReturnType<typeof check>>);
    renderSettings();
    expect(
      await screen.findByRole('button', { name: 'About, update available' })
    ).toHaveTextContent('Update');
  });

  it('stays hidden when the update check fails', async () => {
    mockCheck.mockRejectedValue(new Error('offline'));
    renderSettings();
    await waitFor(() => expect(mockCheck).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('Update')).not.toBeInTheDocument();
  });

  it('checks only once per session across remounts', async () => {
    mockCheck.mockResolvedValue(null);
    renderSettings().unmount();
    renderSettings();
    await waitFor(() => expect(mockCheck).toHaveBeenCalledTimes(1));
  });
});
