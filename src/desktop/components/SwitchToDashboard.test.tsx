import { render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import SwitchToDashboard from './SwitchToDashboard';

const state = vi.hoisted(() => ({ switching: false }));
vi.mock('../context/ProjectContext', () => ({ useProjects: () => state }));

function LocationProbe() {
  const { pathname } = useLocation();
  return <span data-testid="location">{pathname}</span>;
}

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <SwitchToDashboard />
      <LocationProbe />
    </MemoryRouter>
  );
}

describe('SwitchToDashboard', () => {
  it('opens the dashboard when a project switch starts', () => {
    state.switching = false;
    const view = renderAt('/config');
    state.switching = true;
    view.rerender(
      <MemoryRouter initialEntries={['/config']}>
        <SwitchToDashboard />
        <LocationProbe />
      </MemoryRouter>
    );
    expect(screen.getByTestId('location')).toHaveTextContent(/^\/$/);
  });

  it('leaves the page alone when no switch is running', () => {
    state.switching = false;
    renderAt('/config');
    expect(screen.getByTestId('location')).toHaveTextContent('/config');
  });
});
