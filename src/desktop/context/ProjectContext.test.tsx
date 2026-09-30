import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ProjectProvider, useProjects } from './ProjectContext';
import { tauriBridge } from '../api/tauri-bridge';

vi.mock('../api/tauri-bridge', () => ({
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
const global = { id: 'g', name: 'Global', path: '/g', projectType: 'global' as const };
const repo = { id: 'r', name: 'Repo', path: '/r', projectType: 'project' as const };

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
