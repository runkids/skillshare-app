import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Project } from '../../api/tauri-bridge';
import ProjectSettings from './ProjectSettings';

const addedAt = '2026-01-01T00:00:00Z';
const global: Project = {
  id: 'g',
  name: 'Global',
  path: '/Users/me/.config/skillshare',
  projectType: 'global',
  addedAt,
};
const repo: Project = {
  id: 'r',
  name: 'repo',
  path: '/Users/me/code/repo',
  projectType: 'project',
  addedAt,
};

const mocks = vi.hoisted(() => ({
  active: 'g',
  removeProject: vi.fn(() => Promise.resolve()),
  switchWithRestart: vi.fn(() => Promise.resolve(undefined)),
}));

vi.mock('../../context/ProjectContext', () => ({
  useProjects: () => ({
    projects: [global, repo],
    activeProject: mocks.active === 'g' ? global : repo,
    addProject: vi.fn(),
    removeProject: mocks.removeProject,
    switchWithRestart: mocks.switchWithRestart,
    switching: false,
  }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ revealItemInDir: vi.fn() }));

function renderPage() {
  return render(
    <MemoryRouter>
      <ProjectSettings />
    </MemoryRouter>
  );
}

beforeEach(() => {
  mocks.active = 'g';
  vi.clearAllMocks();
});

describe('ProjectSettings', () => {
  it('asks before removing a project and keeps it until confirmed', async () => {
    renderPage();
    fireEvent.click(screen.getByRole('button', { name: 'Remove repo' }));
    expect(mocks.removeProject).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(mocks.removeProject).toHaveBeenCalledWith('r'));
  });

  it('can switch back to Global from a project', async () => {
    mocks.active = 'r';
    renderPage();
    fireEvent.click(screen.getByRole('button', { name: 'Switch' }));
    await waitFor(() => expect(mocks.switchWithRestart).toHaveBeenCalledWith('g'));
  });
});
