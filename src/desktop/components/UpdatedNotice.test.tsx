import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import UpdatedNotice from './UpdatedNotice';
import { JUST_UPDATED_KEY } from '../hooks/useAppUpdate';

vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.7.7') }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));

beforeEach(() => localStorage.clear());

describe('UpdatedNotice', () => {
  it('says which version the restart brought and what changed', async () => {
    localStorage.setItem(
      JUST_UPDATED_KEY,
      JSON.stringify({ version: '0.7.7', notes: '* faster sync' })
    );
    render(<UpdatedNotice />);
    expect(await screen.findByRole('status')).toHaveTextContent(/Updated to v0\.7\.7.*faster sync/);
  });

  it('stays quiet and forgets the note when the update did not apply', async () => {
    localStorage.setItem(JUST_UPDATED_KEY, JSON.stringify({ version: '0.7.8', notes: '' }));
    render(<UpdatedNotice />);
    await waitFor(() => expect(localStorage.getItem(JUST_UPDATED_KEY)).toBeNull());
  });
});
