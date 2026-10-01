import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TitleBar from './TitleBar';
import ProjectDropdown from './ProjectDropdown';

const { reloadView } = vi.hoisted(() => ({ reloadView: vi.fn() }));
const updates = vi.hoisted(() => ({
  cli: null as string | null,
  app: null as string | null,
  skills: [] as string[],
  repositories: [] as string[],
  agents: [] as string[],
  plugins: [] as string[],
}));

vi.mock('../hooks/useUpdates', () => ({ useUpdates: () => updates }));
const screenState = vi.hoisted(() => ({ fullscreen: false }));
vi.mock('../hooks/useFullscreen', () => ({ useFullscreen: () => screenState.fullscreen }));

function LocationProbe() {
  const { pathname, search } = useLocation();
  return <span data-testid="location">{pathname + search}</span>;
}

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
    switching: false,
    reloadView,
  }),
}));

function renderWithUA(ui: React.ReactElement, userAgent: string) {
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(userAgent);
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

afterEach(() => {
  screenState.fullscreen = false;
  updates.cli = null;
  updates.app = null;
  updates.skills = [];
  updates.repositories = [];
  updates.agents = [];
  updates.plugins = [];
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

  it('drops the traffic-light gap in macOS full screen', () => {
    screenState.fullscreen = true;
    const { container } = renderWithUA(<TitleBar />, MAC_UA);
    expect(container.firstChild).toHaveStyle({ paddingLeft: '12px' });
  });

  it('shows the app mark with tight padding on Windows/Linux', () => {
    const { container } = renderWithUA(<TitleBar />, WIN_UA);
    expect(container.firstChild).toHaveStyle({ paddingLeft: '12px' });
    expect(screen.getByTestId('app-mark')).toHaveTextContent('skillshare');
  });
});

describe('TitleBar reload', () => {
  it('asks the web view to reload', () => {
    renderWithUA(<TitleBar />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: 'Reload' }));
    expect(reloadView).toHaveBeenCalledTimes(1);
  });
});

describe('TitleBar update badge', () => {
  it('opens the CLI settings tab when a newer CLI is known', () => {
    updates.cli = 'v0.24.0';
    renderWithUA(
      <>
        <TitleBar />
        <LocationProbe />
      </>,
      MAC_UA
    );
    fireEvent.click(screen.getByRole('button', { name: 'Settings, update available' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/settings?tab=cli');
  });
});

describe('TitleBar skill updates badge', () => {
  it('is hidden when no skill has an update', () => {
    renderWithUA(<TitleBar />, MAC_UA);
    expect(screen.queryByRole('button', { name: /\d+ updates?/ })).not.toBeInTheDocument();
  });

  it('opens the Web UI updates tab with the count shown', () => {
    updates.skills = ['pdf', 'xlsx'];
    renderWithUA(
      <>
        <TitleBar />
        <LocationProbe />
      </>,
      MAC_UA
    );
    fireEvent.click(screen.getByRole('button', { name: '2 updates' }));
    fireEvent.click(screen.getByRole('link', { name: /2 skills/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/skills?tab=updates');
  });
});

describe('TitleBar combined resource updates', () => {
  it('counts every kind once and links each list to its review page', () => {
    updates.skills = ['pdf', 'xlsx', 'docx'];
    updates.plugins = ['hud'];
    updates.agents = ['tutor'];
    updates.repositories = ['_team'];
    renderWithUA(
      <>
        <TitleBar />
        <LocationProbe />
      </>,
      MAC_UA
    );
    const badge = screen.getByRole('button', { name: '6 updates' });
    expect(badge).toHaveAttribute(
      'title',
      '3 skills, 1 plugin, 1 agent, 1 repository have updates'
    );
    fireEvent.click(badge);
    expect(screen.getByRole('link', { name: /3 skills/ })).toHaveAttribute(
      'href',
      '/skills?tab=updates'
    );
    expect(screen.getByRole('link', { name: /1 repository/ })).toHaveAttribute(
      'href',
      '/skills?tab=updates'
    );
    expect(screen.getByRole('link', { name: /1 agent/ })).toHaveAttribute(
      'href',
      '/agents?tab=updates'
    );
    fireEvent.click(screen.getByRole('link', { name: /1 plugin/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/plugins');
    expect(screen.queryByRole('link', { name: /1 plugin/ })).not.toBeInTheDocument();
  });

  it('shows plugin-only updates and dismisses the list on Escape', () => {
    updates.plugins = ['hud'];
    renderWithUA(<TitleBar />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: '1 update' }));
    expect(screen.getByRole('link', { name: /1 plugin/ })).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('link', { name: /1 plugin/ })).not.toBeInTheDocument();
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

  it('closes when focus moves into the CLI iframe', () => {
    vi.stubGlobal('matchMedia', (query: string) => ({ matches: query.includes('reduce') }));
    renderWithUA(<ProjectDropdown />, MAC_UA);
    fireEvent.click(screen.getByRole('button', { name: /Global/ }));
    fireEvent.blur(window);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
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
