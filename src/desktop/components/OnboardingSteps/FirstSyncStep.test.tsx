import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import FirstSyncStep from './FirstSyncStep';
import { tauriBridge } from '../../api/tauri-bridge';

vi.mock('../../api/tauri-bridge', () => ({
  tauriBridge: { runCli: vi.fn(), cliTerminalAccess: vi.fn(), linkCliForTerminal: vi.fn() },
}));

const PATH_COMMAND = `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc`;

beforeEach(() => {
  vi.mocked(tauriBridge.runCli).mockResolvedValue('{}');
});

describe('FirstSyncStep terminal hint', () => {
  it('shows the PATH command when the CLI folder is not on the terminal PATH', async () => {
    vi.mocked(tauriBridge.cliTerminalAccess).mockResolvedValue({
      needsLink: false,
      linked: false,
      pathHint: { dir: '~/.local/bin', command: PATH_COMMAND },
    });
    render(<FirstSyncStep cliPath="/h/.local/bin/skillshare" onComplete={() => {}} />);
    expect(await screen.findByText(PATH_COMMAND, {}, { timeout: 3000 })).toBeTruthy();
  });

  it('keeps the plain terminal tip when the CLI is already on PATH', async () => {
    vi.mocked(tauriBridge.cliTerminalAccess).mockResolvedValue({
      needsLink: false,
      linked: false,
      pathHint: null,
    });
    render(<FirstSyncStep cliPath="/opt/homebrew/bin/skillshare" onComplete={() => {}} />);
    expect(await screen.findByText('skillshare sync', {}, { timeout: 3000 })).toBeTruthy();
  });

  it('links the app-downloaded CLI, then shows the PATH command if still needed', async () => {
    vi.mocked(tauriBridge.cliTerminalAccess).mockResolvedValue({
      needsLink: true,
      linked: false,
      pathHint: null,
    });
    vi.mocked(tauriBridge.linkCliForTerminal).mockResolvedValue({
      needsLink: false,
      linked: true,
      pathHint: { dir: '~/.local/bin', command: PATH_COMMAND },
    });
    render(<FirstSyncStep cliPath="/app/bin/skillshare" onComplete={() => {}} />);
    fireEvent.click(
      await screen.findByRole('button', { name: 'Use in terminal' }, { timeout: 3000 })
    );
    expect(await screen.findByText(PATH_COMMAND)).toBeTruthy();
  });
});
