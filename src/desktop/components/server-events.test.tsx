import { act, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import CliWebView from './CliWebView';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
  refresh: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (e: { payload: unknown }) => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));
vi.mock('../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { serverPort: 19420 }, refresh: mocks.refresh }),
}));
vi.mock('../api/tauri-bridge', () => ({
  tauriBridge: { healthCheck: vi.fn().mockResolvedValue(true) },
}));
vi.mock('../context/ProjectContext', () => ({
  useProjects: () => ({ switching: false, activeProject: null, reloadKey: 0 }),
}));

function emit(event: string, payload?: unknown) {
  act(() => mocks.handlers.get(event)?.({ payload }));
}

beforeEach(() => {
  mocks.handlers.clear();
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
  render(
    <MemoryRouter>
      <CliWebView />
    </MemoryRouter>
  );
});

describe('server supervision events', () => {
  it('points the web view at the port a restarted server got', () => {
    emit('server-restarted', 19421);
    expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
      'src',
      expect.stringContaining('http://localhost:19421')
    );
  });

  it('shows the server as down when supervision gives up', () => {
    emit('server-stopped');
    expect(screen.getByRole('alert', { name: 'Disconnected' })).toBeInTheDocument();
  });
});
