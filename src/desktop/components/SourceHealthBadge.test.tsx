import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SourceHealthBadge from './SourceHealthBadge';
import type { SourceHealth } from '../api/tauri-bridge';

const health = vi.hoisted(() => ({
  current: { localSkills: [], outOfSyncTargets: [], git: null } as SourceHealth,
}));
vi.mock('../hooks/useSourceHealth', () => ({ useSourceHealth: () => health.current }));

function LocationProbe() {
  const { pathname } = useLocation();
  return <span data-testid="location">{pathname}</span>;
}

function renderBadge() {
  return render(
    <MemoryRouter>
      <SourceHealthBadge />
      <LocationProbe />
    </MemoryRouter>
  );
}

afterEach(() => {
  health.current = { localSkills: [], outOfSyncTargets: [], git: null };
});

describe('SourceHealthBadge', () => {
  it('is hidden when the source is healthy', () => {
    health.current = { ...health.current, git: { uncommitted: 0, ahead: 0, behind: 0 } };
    renderBadge();
    expect(screen.queryByRole('button', { name: /Source health/ })).not.toBeInTheDocument();
  });

  it('summarizes each finding', () => {
    health.current = {
      localSkills: ['mine'],
      outOfSyncTargets: ['claude', 'codex'],
      git: { uncommitted: 1, ahead: 2, behind: 4 },
    };
    renderBadge();
    expect(screen.getByRole('button', { name: /Source health/ })).toHaveTextContent(
      '1 local · 2 out of sync · ↑3 · ↓4'
    );
  });

  it('opens the Web UI sync page for target drift', () => {
    health.current = { ...health.current, localSkills: ['mine'] };
    renderBadge();
    fireEvent.click(screen.getByRole('button', { name: /Source health/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/sync');
  });

  it('opens the Web UI git page when only the remote differs', () => {
    health.current = { ...health.current, git: { uncommitted: 0, ahead: 0, behind: 1 } };
    renderBadge();
    fireEvent.click(screen.getByRole('button', { name: /Source health/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/git');
  });
});
