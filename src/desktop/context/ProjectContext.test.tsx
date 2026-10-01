import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ProjectProvider, useProjects } from './ProjectContext';
import { tauriBridge, TRAY_PROJECT_REQUESTED_EVENT, type Project } from '../api/tauri-bridge';

const events = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: string }) => void>(),
  off: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (event: { payload: string }) => void) => {
    events.handlers.set(event, handler);
    return Promise.resolve(events.off);
  },
}));

vi.mock('../api/tauri-bridge', () => ({
  TRAY_PROJECT_REQUESTED_EVENT: 'tray-project-requested',
  tauriBridge: {
    listProjects: vi.fn(),
    getActiveProject: vi.fn(),
    removeProject: vi.fn(),
    switchProject: vi.fn(),
    stopServer: vi.fn(),
    startServer: vi.fn(),
    detectCli: vi.fn(),
  },
}));

const bridge = vi.mocked(tauriBridge);
const addedAt = '2026-01-01T00:00:00Z';
const global: Project = { id: 'g', name: 'Global', path: '/g', projectType: 'global', addedAt };
const repo: Project = { id: 'r', name: 'Repo', path: '/r', projectType: 'project', addedAt };

const wrapper = ({ children }: { children: ReactNode }) => (
  <ProjectProvider>{children}</ProjectProvider>
);

async function renderProjects() {
  const hook = renderHook(() => useProjects(), { wrapper });
  await act(async () => {});
  return hook;
}

beforeEach(() => {
  vi.clearAllMocks();
  events.handlers.clear();
  bridge.listProjects.mockResolvedValue([global, repo]);
  bridge.detectCli.mockResolvedValue('/bin/skillshare');
  bridge.startServer.mockResolvedValue(19420);
});

describe('removeProject', () => {
  it('restarts the server for the newly active project when the active one is removed', async () => {
    bridge.getActiveProject.mockResolvedValueOnce(repo).mockResolvedValue(global);
    const { result } = await renderProjects();
    bridge.getActiveProject.mockResolvedValueOnce(repo).mockResolvedValue(global);

    await act(() => result.current.removeProject('r'));

    expect(bridge.startServer).toHaveBeenCalledWith('/bin/skillshare', '/g');
  });

  it('leaves the server alone when a non-active project is removed', async () => {
    bridge.getActiveProject.mockResolvedValue(global);
    const { result } = await renderProjects();

    await act(() => result.current.removeProject('r'));

    expect(bridge.startServer).not.toHaveBeenCalled();
  });

  it('stops the server when the last project is removed', async () => {
    bridge.getActiveProject.mockResolvedValue(global);
    const { result } = await renderProjects();
    bridge.getActiveProject.mockResolvedValueOnce(global).mockResolvedValue(null);

    await act(() => result.current.removeProject('g'));

    expect(bridge.stopServer).toHaveBeenCalledTimes(1);
  });
});

describe('tray project selection', () => {
  it('uses the in-app stop/switch/start sequence and refreshes the active project', async () => {
    bridge.getActiveProject.mockResolvedValue(global);
    const { result, unmount } = await renderProjects();
    bridge.getActiveProject.mockResolvedValue(repo);
    await act(async () => {
      events.handlers.get(TRAY_PROJECT_REQUESTED_EVENT)?.({ payload: 'r' });
    });
    expect(bridge.stopServer).toHaveBeenCalledTimes(1);
    expect(bridge.switchProject).toHaveBeenCalledWith('r');
    expect(bridge.startServer).toHaveBeenCalledWith('/bin/skillshare', '/r');
    expect(bridge.stopServer.mock.invocationCallOrder[0]).toBeLessThan(
      bridge.switchProject.mock.invocationCallOrder[0]
    );
    expect(bridge.switchProject.mock.invocationCallOrder[0]).toBeLessThan(
      bridge.startServer.mock.invocationCallOrder[0]
    );
    expect(result.current.activeProject?.id).toBe('r');
    unmount();
    await act(async () => {});
    expect(events.off).toHaveBeenCalled();
  });

  it('ignores the active project and overlapping requests', async () => {
    bridge.getActiveProject.mockResolvedValue(global);
    await renderProjects();
    await act(async () => {
      events.handlers.get(TRAY_PROJECT_REQUESTED_EVENT)?.({ payload: 'g' });
    });
    expect(bridge.stopServer).not.toHaveBeenCalled();
    bridge.getActiveProject.mockResolvedValue(repo);
    await act(async () => {
      const handler = events.handlers.get(TRAY_PROJECT_REQUESTED_EVENT);
      handler?.({ payload: 'r' });
      handler?.({ payload: 'r' });
    });
    expect(bridge.stopServer).toHaveBeenCalledTimes(1);
    expect(bridge.switchProject).toHaveBeenCalledTimes(1);
  });
});
