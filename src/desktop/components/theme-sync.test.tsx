import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '../../context/ThemeContext';
import { useTheme } from '../../context/useTheme';
import { tauriBridge } from '../api/tauri-bridge';
import CliWebView from './CliWebView';

const { refresh } = vi.hoisted(() => ({ refresh: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { serverPort: 19420 }, refresh }),
}));
vi.mock('../api/tauri-bridge', () => ({
  tauriBridge: { setPreferredTheme: vi.fn().mockResolvedValue(undefined) },
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

function setup() {
  render(
    <MemoryRouter>
      <ThemeProvider>
        <Controls />
        <CliWebView />
      </ThemeProvider>
    </MemoryRouter>
  );
  const iframe = screen.getByTitle('skillshare UI') as HTMLIFrameElement;
  const post = vi.spyOn(iframe.contentWindow!, 'postMessage');
  fireEvent.load(iframe);
  const receive = (theme: string) =>
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', {
          source: iframe.contentWindow,
          data: { type: 'theme-change', theme },
        })
      );
    });
  return { iframe, post, receive };
}

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
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
    expect(tauriBridge.setPreferredTheme).toHaveBeenLastCalledWith('system');
  });

  it('keeps System preference when the iframe echoes its resolved mode', () => {
    const { receive } = setup();
    fireEvent.click(screen.getByText('System'));
    receive('light');
    expect(screen.getByText('clean/light/system')).toBeInTheDocument();
    expect(tauriBridge.setPreferredTheme).toHaveBeenLastCalledWith('system');
  });
});
