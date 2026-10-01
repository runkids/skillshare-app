import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ServerStatus from './ServerStatus';
import { resetServerStoppedForTests } from '../hooks/useServerStopped';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, () => void>(),
  appInfo: { serverRunning: true, serverPort: 19420 } as {
    serverRunning: boolean;
    serverPort: number | null;
  },
  reloadView: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: () => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));
vi.mock('../context/TauriContext', () => ({ useTauri: () => ({ appInfo: mocks.appInfo }) }));
vi.mock('../context/ProjectContext', () => ({
  useProjects: () => ({ reloadView: mocks.reloadView }),
}));

function emit(event: string) {
  act(() => mocks.handlers.get(event)?.());
}

beforeEach(() => {
  mocks.handlers.clear();
  resetServerStoppedForTests();
  mocks.reloadView.mockClear();
  mocks.appInfo = { serverRunning: true, serverPort: 19420 };
});

describe('ServerStatus', () => {
  it('shows only a labelled dot while the server runs', () => {
    render(<ServerStatus />);
    expect(screen.getByRole('img', { name: 'Server running on port 19420' })).toBeInTheDocument();
  });

  it('renders nothing before the server has started', () => {
    mocks.appInfo = { serverRunning: false, serverPort: null };
    const { container } = render(<ServerStatus />);
    expect(container).toBeEmptyDOMElement();
  });

  it('says the server stopped when the supervisor gives up', () => {
    render(<ServerStatus />);
    emit('server-stopped');
    expect(screen.getByRole('button', { name: /Server stopped/ })).toBeInTheDocument();
  });

  it('restarts through the header reload when clicked', () => {
    render(<ServerStatus />);
    emit('server-stopped');
    fireEvent.click(screen.getByRole('button', { name: /Server stopped/ }));
    expect(mocks.reloadView).toHaveBeenCalled();
  });

  it('goes back to the dot after a restart', () => {
    render(<ServerStatus />);
    emit('server-stopped');
    emit('server-restarted');
    expect(screen.getByRole('img', { name: /Server running/ })).toBeInTheDocument();
  });
});
