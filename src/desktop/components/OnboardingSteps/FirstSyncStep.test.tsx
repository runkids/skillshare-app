import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import FirstSyncStep from './FirstSyncStep';
import { tauriBridge } from '../../api/tauri-bridge';

vi.mock('../../api/tauri-bridge', () => ({
  tauriBridge: { runCli: vi.fn(), cliPathHint: vi.fn() },
}));

const PATH_COMMAND = `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc`;

beforeEach(() => {
  vi.mocked(tauriBridge.runCli).mockResolvedValue('{}');
});

describe('FirstSyncStep terminal hint', () => {
  it('shows the PATH command when the CLI folder is not on the terminal PATH', async () => {
    vi.mocked(tauriBridge.cliPathHint).mockResolvedValue({
      dir: '~/.local/bin',
      command: PATH_COMMAND,
    });
    render(<FirstSyncStep cliPath="/h/.local/bin/skillshare" onComplete={() => {}} />);
    expect(await screen.findByText(PATH_COMMAND, {}, { timeout: 3000 })).toBeTruthy();
  });

  it('keeps the plain terminal tip when the CLI is already on PATH', async () => {
    vi.mocked(tauriBridge.cliPathHint).mockResolvedValue(null);
    render(<FirstSyncStep cliPath="/opt/homebrew/bin/skillshare" onComplete={() => {}} />);
    expect(await screen.findByText('skillshare sync', {}, { timeout: 3000 })).toBeTruthy();
  });
});
