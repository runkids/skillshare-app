import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import QuickActionsPalette from './QuickActionsPalette';
import { sourceFromQuery } from './useQuickActions';

const mocks = vi.hoisted(() => ({
  context: vi.fn(),
  search: vi.fn(),
  install: vi.fn(),
  close: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false }));
vi.mock('../../api/tauri-bridge', () => ({
  QUICK_ACTIONS_OPENED_EVENT: 'quick-actions-opened',
  tauriBridge: {
    getQuickActionsContext: mocks.context,
    quickSearch: mocks.search,
    quickInstall: mocks.install,
    closeQuickActions: mocks.close,
  },
}));
const result = { name: 'pdf', description: 'Read PDFs', source: 'org/repo/skills/pdf', skill: '' };

beforeEach(() => {
  vi.resetAllMocks();
  mocks.context.mockResolvedValue({ projectId: 'p1', projectName: 'Work', sourceDir: '/skills' });
  mocks.search.mockResolvedValue([result]);
  mocks.install.mockResolvedValue('Installed pdf');
  mocks.close.mockResolvedValue(undefined);
});
afterEach(() => vi.useRealTimers());

async function ready() {
  render(<QuickActionsPalette />);
  await screen.findByText('Work · /skills');
  return screen.getByRole('textbox', { name: 'Search skills' });
}

async function search() {
  const input = await ready();
  vi.useFakeTimers();
  fireEvent.change(input, { target: { value: 'pdf' } });
  await act(() => vi.advanceTimersByTimeAsync(350));
  vi.useRealTimers();
  return input;
}

describe('Quick Actions', () => {
  it('focuses search when ready and selects results with arrow keys', async () => {
    mocks.search.mockResolvedValueOnce([
      result,
      { ...result, name: 'review', source: 'org/repo/skills/review' },
    ]);
    const input = await search();
    expect(input).toHaveFocus();
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    fireEvent.submit(input.closest('form')!);
    expect(screen.getByText('Install review?')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.getByRole('textbox', { name: 'Search skills' })).toHaveFocus();
  });

  it('debounces searches and ignores stale replies', async () => {
    const input = await ready();
    vi.useFakeTimers();
    let resolveOld!: (value: (typeof result)[]) => void;
    mocks.search.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveOld = resolve;
        })
    );
    fireEvent.change(input, { target: { value: 'old' } });
    await act(() => vi.advanceTimersByTimeAsync(349));
    expect(mocks.search).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(1));
    fireEvent.change(input, { target: { value: 'pdf' } });
    await act(() => vi.advanceTimersByTimeAsync(350));
    await act(async () => resolveOld([{ ...result, name: 'stale' }]));
    expect(screen.queryByText('stale')).not.toBeInTheDocument();
    expect(screen.getByText('pdf')).toBeInTheDocument();
    expect(mocks.search).toHaveBeenLastCalledWith('pdf', 'p1');
    fireEvent.change(input, { target: { value: '' } });
    expect(screen.queryByText('pdf')).not.toBeInTheDocument();
  });

  it('reviews the exact source before installing and shows progress and output', async () => {
    const input = await search();
    fireEvent.submit(input.closest('form')!);
    expect(mocks.install).not.toHaveBeenCalled();
    expect(screen.getByText('Install pdf?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install' })).toHaveFocus();
    expect(screen.getByText(result.source)).toBeInTheDocument();
    let complete!: (output: string) => void;
    mocks.install.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          complete = resolve;
        })
    );
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));
    expect(mocks.install).toHaveBeenCalledWith(result.source, '', 'p1');
    expect(screen.getByText('Installing skill…')).toBeInTheDocument();
    await act(async () => complete('Installed pdf'));
    expect(screen.getByText('Installed pdf')).toBeInTheDocument();
  });

  it('installs a pasted owner/repo source without searching', async () => {
    const input = await ready();
    fireEvent.change(input, { target: { value: 'anthropics/skills' } });
    fireEvent.submit(input.closest('form')!);
    expect(screen.getByText('Install anthropics/skills?')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));
    await screen.findByText('Installed pdf');
    expect(mocks.install).toHaveBeenCalledWith('anthropics/skills', '', 'p1');
    expect(mocks.search).not.toHaveBeenCalled();
  });

  it('reports a failed installation and closes on Escape', async () => {
    await search();
    mocks.install.mockRejectedValueOnce('Audit blocked installation');
    fireEvent.click(screen.getByRole('button', { name: /pdf/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Audit blocked installation');
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(mocks.close).toHaveBeenCalledOnce());
  });
});

describe('sourceFromQuery', () => {
  it.each([
    'https://github.com/org/repo',
    'git@github.com:org/repo.git',
    'org/repo',
    'org/repo/skills/pdf',
  ])('treats %s as a source', (query) => {
    expect(sourceFromQuery(query)).toBe(query);
  });

  it.each(['pdf', 'pdf reader', '-x/y', ''])('treats %s as search words', (query) => {
    expect(sourceFromQuery(query)).toBeNull();
  });
});
