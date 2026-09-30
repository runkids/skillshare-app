import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { tauriBridge } from '../api/tauri-bridge';
import TitleBar from './TitleBar';
import CliWebView from './CliWebView';

vi.mock('../context/TerminalContext', () => ({
  useTerminal: () => ({
    activeView: 'webui',
    setActiveView: vi.fn(),
    hasUnreadAny: false,
  }),
}));

describe('browser preview without the Tauri runtime', () => {
  it('renders the title bar without accessing native window metadata', () => {
    render(
      <MemoryRouter>
        <TitleBar />
      </MemoryRouter>
    );

    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
  });

  it('explains the desktop requirement without trying to start the CLI', () => {
    const detectCli = vi.spyOn(tauriBridge, 'detectCli');
    render(
      <MemoryRouter>
        <CliWebView />
      </MemoryRouter>
    );

    expect(screen.getByText('Browser preview')).toBeInTheDocument();
    expect(detectCli).not.toHaveBeenCalled();
    detectCli.mockRestore();
  });
});
