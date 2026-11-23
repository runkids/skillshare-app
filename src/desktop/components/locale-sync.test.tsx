import { act, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '../../context/ThemeContext';
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

function Tree() {
  return (
    <MemoryRouter initialEntries={['/']}>
      <ThemeProvider>
        <CliWebView />
      </ThemeProvider>
    </MemoryRouter>
  );
}

const frameUrl = () =>
  new URL((screen.getByTitle('skillshare UI') as HTMLIFrameElement).getAttribute('src')!);

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
});

describe('CLI UI language across launches', () => {
  it('opens without ?lang= before the user picks a language', () => {
    render(<Tree />);
    expect(frameUrl().searchParams.has('lang')).toBe(false);
  });

  it('keeps the language the CLI UI reports and passes it on the next launch', () => {
    const { unmount } = render(<Tree />);
    const iframe = screen.getByTitle('skillshare UI') as HTMLIFrameElement;
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', {
          source: iframe.contentWindow,
          data: { type: 'locale-change', locale: 'en' },
        })
      );
    });
    unmount();

    render(<Tree />);
    expect(frameUrl().searchParams.get('lang')).toBe('en');
  });

  it('ignores language reports from other windows and malformed values', () => {
    const { unmount } = render(<Tree />);
    const iframe = screen.getByTitle('skillshare UI') as HTMLIFrameElement;
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', {
          source: window,
          data: { type: 'locale-change', locale: 'en' },
        })
      );
      window.dispatchEvent(
        new MessageEvent('message', {
          source: iframe.contentWindow,
          data: { type: 'locale-change', locale: 'en"><script>' },
        })
      );
    });
    unmount();

    render(<Tree />);
    expect(frameUrl().searchParams.has('lang')).toBe(false);
  });
});
