import { act, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivityEntry } from '../api/tauri-bridge';
import ActivityPage from './ActivityPage';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, () => void>(),
  getActivity: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: () => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));
vi.mock('../api/tauri-bridge', () => ({
  SYNC_COMPLETED_EVENT: 'sync-completed',
  tauriBridge: { getActivity: mocks.getActivity },
}));
vi.mock('../context/ProjectContext', () => ({
  useProjects: () => ({ activeProject: { id: 'g', name: 'Global' } }),
}));

const entries: ActivityEntry[] = [
  {
    ts: new Date().toISOString(),
    cmd: 'install',
    status: 'error',
    message: 'permission denied',
    durationMs: 4655,
    subjects: ['runkids/skillshare'],
    detail: null,
  },
  {
    ts: new Date(Date.now() - 2 * 86_400_000).toISOString(),
    cmd: 'sync',
    status: 'ok',
    message: null,
    durationMs: 41,
    subjects: [],
    detail: '7 targets',
  },
];

beforeEach(async () => {
  mocks.handlers.clear();
  mocks.getActivity.mockReset().mockResolvedValue(entries);
  await act(async () => {
    render(
      <MemoryRouter>
        <ActivityPage />
      </MemoryRouter>
    );
  });
});

describe('ActivityPage', () => {
  it('groups today’s entries under Today', () => {
    expect(
      within(screen.getByRole('region', { name: 'Today' })).getByText('runkids/skillshare')
    ).toBeInTheDocument();
  });

  it('highlights errors with their message', () => {
    expect(screen.getByText('permission denied').closest('li')).toHaveAttribute(
      'data-status',
      'error'
    );
  });

  it('re-reads the log after a sync', async () => {
    await act(async () => mocks.handlers.get('sync-completed')?.());
    expect(mocks.getActivity).toHaveBeenCalledTimes(2);
  });
});
