import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '../../context/ThemeContext';
import { useTheme } from '../../context/useTheme';
import CliWebView from './CliWebView';

const { refresh, project } = vi.hoisted(() => ({
  refresh: vi.fn(),
  project: { reloadKey: 0, switching: false },
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock('../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { serverPort: 19420, cliVersion: 'v0.30.7' }, refresh }),
}));
vi.mock('../api/tauri-bridge', () => ({
  tauriBridge: {
    healthCheck: vi.fn().mockResolvedValue(true),
  },
}));
vi.mock('../context/ProjectContext', () => ({
  useProjects: () => ({
    switching: project.switching,
    activeProject: null,
    reloadKey: project.reloadKey,
  }),
}));

function Controls() {
  const { style, resolvedMode, modePreference, setModePreference } = useTheme();
  return (
    <>
      <output>{`${style}/${resolvedMode}/${modePreference}`}</output>
      <button onClick={() => setModePreference('dark')}>Dark</button>
      <button onClick={() => setModePreference('system')}>System</button>
    </>
  );
}

function Tree({ path = '/' }: { path?: string }) {
  return (
    <MemoryRouter initialEntries={[path]}>
      <ThemeProvider>
        <Controls />
        <CliWebView />
      </ThemeProvider>
    </MemoryRouter>
  );
}

// By default the CLI UI has finished starting, so its reports are the user's choices.
function setup({ started = true } = {}) {
  const { rerender } = render(<Tree />);
  const iframe = screen.getByTitle('skillshare UI') as HTMLIFrameElement;
  const post = vi.spyOn(iframe.contentWindow!, 'postMessage');
  fireEvent.load(iframe);
  if (started) vi.advanceTimersByTime(2000);
  const receive = (theme: string) =>
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', {
          source: iframe.contentWindow,
          data: { type: 'theme-change', theme },
        })
      );
    });
  return { iframe, post, receive, rerender };
}

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  vi.useFakeTimers({ toFake: ['Date'] });
  project.reloadKey = 0;
  project.switching = false;
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
});

afterEach(() => {
  vi.useRealTimers();
});

describe('desktop and CLI theme synchronization', () => {
  it('restores Playful and pushes both axes after iframe load and mode changes', () => {
    localStorage.setItem('skillshare-style', 'playful');
    const { post, iframe } = setup();
    expect(post).toHaveBeenLastCalledWith(
      { type: 'theme-push', mode: 'light', style: 'playful' },
      '*'
    );
    fireEvent.click(screen.getByText('Dark'));
    expect(post).toHaveBeenLastCalledWith(
      { type: 'theme-push', mode: 'dark', style: 'playful' },
      '*'
    );
    post.mockClear();
    fireEvent.load(iframe);
    expect(post).toHaveBeenCalledWith({ type: 'theme-push', mode: 'dark', style: 'playful' }, '*');
  });

  it('changes style and mode independently without dropping rapid messages', () => {
    const { receive } = setup();
    receive('playful');
    receive('dark');
    expect(screen.getByText('playful/dark/dark')).toBeInTheDocument();
    receive('clean');
    expect(screen.getByText('clean/dark/dark')).toBeInTheDocument();
    receive('light');
    expect(screen.getByText('clean/light/light')).toBeInTheDocument();
  });

  it('keeps Dark + Clean when the CLI UI starts in Playful', () => {
    localStorage.setItem('skillshare-theme-preference', 'dark');
    const { post, receive } = setup({ started: false });
    post.mockClear();
    receive('playful');
    expect(screen.getByText('clean/dark/dark')).toBeInTheDocument();
    expect(post).toHaveBeenCalledWith({ type: 'theme-push', mode: 'dark', style: 'clean' }, '*');
  });

  it('keeps Playful dark when the CLI UI starts in light mode', () => {
    localStorage.setItem('skillshare-style', 'playful');
    localStorage.setItem('skillshare-theme-preference', 'dark');
    const { post, receive } = setup({ started: false });
    post.mockClear();
    receive('light');
    expect(screen.getByText('playful/dark/dark')).toBeInTheDocument();
    expect(post).toHaveBeenCalledWith({ type: 'theme-push', mode: 'dark', style: 'playful' }, '*');
  });

  it('keeps the saved style when the CLI UI reports before the iframe load event', () => {
    render(<Tree />);
    const iframe = screen.getByTitle('skillshare UI') as HTMLIFrameElement;
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', {
          source: iframe.contentWindow,
          data: { type: 'theme-change', theme: 'playful' },
        })
      );
    });
    expect(screen.getByText('clean/light/light')).toBeInTheDocument();
  });

  it('keeps the saved style when the CLI UI reloads itself and starts in Playful', () => {
    const { iframe, receive } = setup();
    fireEvent.load(iframe);
    receive('playful');
    expect(screen.getByText('clean/light/light')).toBeInTheDocument();
  });

  it('keeps the saved style when the CLI UI announces a new page and starts in Playful', () => {
    const { iframe, receive } = setup();
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', { source: iframe.contentWindow, data: { type: 'theme-init' } })
      );
    });
    receive('playful');
    expect(screen.getByText('clean/light/light')).toBeInTheDocument();
  });

  it('asks the CLI UI for both axes in the iframe URL', () => {
    localStorage.setItem('skillshare-theme-preference', 'dark');
    setup();
    expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
      'src',
      'http://localhost:19420?theme=dark&mode=dark&style=clean'
    );
  });

  it('follows system appearance changes while preserving System preference', () => {
    let dark = false;
    const listeners = new Set<() => void>();
    vi.stubGlobal('matchMedia', () => ({
      get matches() {
        return dark;
      },
      addEventListener: (_event: string, handler: () => void) => listeners.add(handler),
      removeEventListener: (_event: string, handler: () => void) => listeners.delete(handler),
    }));
    const { post } = setup();
    fireEvent.click(screen.getByText('System'));
    act(() => {
      dark = true;
      listeners.forEach((listener) => listener());
    });
    expect(screen.getByText('clean/dark/system')).toBeInTheDocument();
    expect(post).toHaveBeenLastCalledWith(
      { type: 'theme-push', mode: 'dark', style: 'clean' },
      '*'
    );
    expect(localStorage.getItem('skillshare-theme-preference')).toBe('system');
  });

  it('reloads the iframe with the style picked inside it, not the one it first loaded with', async () => {
    localStorage.setItem('skillshare-style', 'playful');
    const { receive, rerender } = setup();
    receive('clean');
    project.reloadKey = 1;
    rerender(<Tree />);
    await waitFor(() =>
      expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
        'src',
        'http://localhost:19420?theme=clean&mode=light&style=clean'
      )
    );
  });

  it('keeps System preference when the iframe echoes its resolved mode', () => {
    const { receive } = setup();
    fireEvent.click(screen.getByText('System'));
    receive('light');
    expect(screen.getByText('clean/light/system')).toBeInTheDocument();
    expect(localStorage.getItem('skillshare-theme-preference')).toBe('system');
  });
});

describe('CLI Web UI page', () => {
  it('opens the page matching the shell path, keeping its query', () => {
    render(<Tree path="/skills?tab=updates" />);
    expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
      'src',
      'http://localhost:19420/skills?tab=updates&theme=clean&mode=light&style=clean'
    );
  });

  function reportRoute(source: MessageEventSource | null, path: string) {
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', { source, data: { type: 'route-change', path } })
      );
    });
  }

  it('reloads on the page the CLI UI reported, not the dashboard', async () => {
    const { iframe, rerender } = setup();
    reportRoute(iframe.contentWindow, '/skills/demo?tab=files');
    project.reloadKey = 1;
    rerender(<Tree />);
    await waitFor(() =>
      expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
        'src',
        'http://localhost:19420/skills/demo?tab=files&theme=clean&mode=light&style=clean'
      )
    );
  });

  it('ignores routes reported by other windows', async () => {
    const { iframe, rerender } = setup();
    reportRoute(window, '/skills');
    project.reloadKey = 1;
    rerender(<Tree />);
    await waitFor(() => expect(screen.getByTitle('skillshare UI')).not.toBe(iframe));
    expect(screen.getByTitle('skillshare UI')).toHaveAttribute(
      'src',
      'http://localhost:19420?theme=clean&mode=light&style=clean'
    );
  });
});

describe('starting card', () => {
  it('shows the CLI version with one v while switching projects', () => {
    project.switching = true;
    render(<Tree />);
    expect(screen.getByText('v0.30.7')).toBeInTheDocument();
  });
});
