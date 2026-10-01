import { act, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import UpdateCheckListener from './UpdateCheckListener';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, () => void>(),
  recheck: vi.fn(),
  checkUpdatesNow: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: () => void) => {
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));
vi.mock('../hooks/useAppUpdate', () => ({ useAppUpdate: () => ({ recheck: mocks.recheck }) }));
vi.mock('../hooks/useUpdates', () => ({
  useUpdates: () => ({ cli: null, app: null }),
  checkUpdatesNow: mocks.checkUpdatesNow,
}));

function LocationProbe() {
  const { pathname, search } = useLocation();
  return <span data-testid="location">{pathname + search}</span>;
}

describe('UpdateCheckListener', () => {
  it('opens About and checks both app and CLI when the menu item is picked', () => {
    render(
      <MemoryRouter>
        <UpdateCheckListener />
        <LocationProbe />
      </MemoryRouter>
    );
    act(() => mocks.handlers.get('check-for-updates')?.());
    expect([
      screen.getByTestId('location').textContent,
      mocks.recheck.mock.calls.length,
      mocks.checkUpdatesNow.mock.calls.length,
    ]).toEqual(['/settings?tab=about', 1, 1]);
  });
});
