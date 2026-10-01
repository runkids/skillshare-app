import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import QuickActionsSettings from './QuickActionsSettings';

const mocks = vi.hoisted(() => ({ get: vi.fn(), set: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('../../api/tauri-bridge', () => ({
  tauriBridge: {
    getQuickActionsSettings: mocks.get,
    setQuickActionsSettings: mocks.set,
  },
}));

beforeEach(() => {
  vi.resetAllMocks();
  mocks.get.mockResolvedValue({ enabled: true, shortcut: 'Control+Shift+K', error: null });
  mocks.set.mockResolvedValue(undefined);
});

describe('Quick Actions settings', () => {
  it('saves a custom shortcut and disabled state', async () => {
    render(<QuickActionsSettings />);
    const toggle = screen.getByRole('switch', { name: 'Quick Actions shortcut' });
    await waitFor(() => expect(toggle).toBeEnabled());
    fireEvent.click(toggle);
    fireEvent.change(screen.getByRole('textbox', { name: 'Keyboard shortcut' }), {
      target: { value: 'Control+Alt+K' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save shortcut' }));
    await screen.findByRole('button', { name: 'Saved' });
    expect(mocks.set).toHaveBeenCalledWith(false, 'Control+Alt+K');
  });

  it('keeps a registration error visible when saving fails', async () => {
    mocks.set.mockRejectedValueOnce('Shortcut already registered');
    render(<QuickActionsSettings />);
    const button = screen.getByRole('button', { name: 'Save shortcut' });
    await waitFor(() => expect(button).toBeEnabled());
    fireEvent.click(button);
    expect(await screen.findByRole('alert')).toHaveTextContent('Shortcut already registered');
    expect(screen.queryByRole('button', { name: 'Saved' })).not.toBeInTheDocument();
  });
});
