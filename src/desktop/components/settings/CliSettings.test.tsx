import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import CliSettings from './CliSettings';

vi.mock('../../api/tauri-bridge', () => ({
  tauriBridge: { detectCli: () => Promise.resolve(null) },
}));
vi.mock('../../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { cliVersion: 'v0.23.0', cliSource: 'homebrew' } }),
}));
vi.mock('../../hooks/useUpdates', () => ({ useUpdates: () => ({ cli: 'v0.24.0', app: null }) }));

function LocationProbe() {
  return <span data-testid="location">{useLocation().search}</span>;
}

describe('CliSettings', () => {
  it('sends a newer CLI to the Updates tab instead of upgrading here', () => {
    render(
      <MemoryRouter>
        <CliSettings />
        <LocationProbe />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByRole('button', { name: 'v0.24.0 available' }));
    expect(screen.getByTestId('location')).toHaveTextContent('?tab=updates');
  });
});
