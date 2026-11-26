import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '../../../context/ThemeContext';
import themes from '../../../context/customTheme.fixture.json';
import AppearanceSettings from './AppearanceSettings';

function renderSettings() {
  render(
    <MemoryRouter initialEntries={['/settings']}>
      <ThemeProvider>
        <Routes>
          <Route path="/settings" element={<AppearanceSettings />} />
          <Route path="/appearance" element={<p>Dashboard theme editor</p>} />
        </Routes>
      </ThemeProvider>
    </MemoryRouter>
  );
}

beforeEach(() => {
  localStorage.clear();
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
});

describe('Appearance settings', () => {
  it('offers only Clean and Playful before a custom theme is made', () => {
    renderSettings();
    const styles = screen.getByRole('radiogroup', { name: 'Style' });
    expect(within(styles).getAllByRole('radio')).toHaveLength(2);
    expect(screen.queryByText('Edit in the dashboard')).not.toBeInTheDocument();
  });

  it('switches back to the custom theme made in the dashboard', () => {
    localStorage.setItem('skillshare-custom-theme', JSON.stringify(themes.pine));
    renderSettings();
    fireEvent.click(screen.getByRole('radio', { name: /Pine/ }));
    expect(localStorage.getItem('skillshare-style')).toBe('custom');
  });

  it('opens the theme editor in the dashboard', () => {
    localStorage.setItem('skillshare-custom-theme', JSON.stringify(themes.pine));
    renderSettings();
    fireEvent.click(screen.getByText('Edit in the dashboard'));
    expect(screen.getByText('Dashboard theme editor')).toBeInTheDocument();
  });
});
