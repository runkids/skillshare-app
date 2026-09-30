import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TitleBar from './TitleBar';
import ProjectDropdown from './ProjectDropdown';

const MAC_UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15';
const WIN_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36';

vi.mock('../context/TerminalContext', () => ({
  useTerminal: () => ({ activeView: 'webui', setActiveView: vi.fn(), hasUnreadAny: false }),
}));
vi.mock('../context/ProjectContext', () => ({
  useProjects: () => ({
    projects: [
      { id: 'g', name: 'Global', path: '/Users/me/.config/skillshare', projectType: 'global' },
    ],
    activeProject: {
      id: 'g',
      name: 'Global',
      path: '/Users/me/.config/skillshare',
      projectType: 'global',
    },
    switchWithRestart: vi.fn(),
  }),
}));

function renderWithUA(ui: React.ReactElement, userAgent: string) {
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(userAgent);
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('TitleBar platform chrome', () => {
  it('leaves room for macOS traffic lights without an app mark', () => {
    const { container } = renderWithUA(<TitleBar />, MAC_UA);
    expect(container.firstChild).toHaveStyle({ paddingLeft: '84px' });
    expect(screen.queryByTestId('app-mark')).not.toBeInTheDocument();
  });

  it('shows the app mark with tight padding on Windows/Linux', () => {
    const { container } = renderWithUA(<TitleBar />, WIN_UA);
    expect(container.firstChild).toHaveStyle({ paddingLeft: '12px' });
    expect(screen.getByTestId('app-mark')).toHaveTextContent('skillshare');
  });
});

describe('ProjectDropdown menu', () => {
  it('explains how to add a project when none exist', () => {
    renderWithUA(<ProjectDropdown />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    expect(
      screen.getByText('No projects yet. Add a repo to give it its own skills.')
    ).toBeInTheDocument();
  });

  it('uses the ⌥ symbol for the new-session hint on macOS', () => {
    renderWithUA(<ProjectDropdown />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    expect(screen.getByText(/click a project to open it in a new session/)).toHaveTextContent(
      '⌥ + click'
    );
  });

  it('uses Alt for the new-session hint elsewhere', () => {
    renderWithUA(<ProjectDropdown />, WIN_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    expect(screen.getByText(/click a project to open it in a new session/)).toHaveTextContent(
      'Alt + click'
    );
  });

  it('keeps the menu mounted while the close animation plays, then unmounts', () => {
    vi.useFakeTimers();
    renderWithUA(<ProjectDropdown />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.getByRole('menu')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('unmounts immediately when reduced motion is preferred', () => {
    vi.stubGlobal('matchMedia', (query: string) => ({ matches: query.includes('reduce') }));
    renderWithUA(<ProjectDropdown />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
});
