import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import StatusButton from './StatusButton';
import type { AuditFinding, AvailableUpdates, SourceHealth } from '../../api/tauri-bridge';

const state = vi.hoisted(() => ({
  findings: [] as AuditFinding[],
  updates: {
    cli: null,
    app: null,
    skills: [],
    repositories: [],
    agents: [],
    plugins: [],
  } as AvailableUpdates,
  health: { outOfSyncTargets: [], git: null } as SourceHealth,
  stopped: false,
}));
const bridge = vi.hoisted(() => ({
  checkStatusNow: vi.fn(() => Promise.resolve()),
}));
const { reloadView } = vi.hoisted(() => ({ reloadView: vi.fn() }));

vi.mock('../../api/tauri-bridge', () => ({ tauriBridge: bridge }));
vi.mock('../../hooks/useAudit', () => ({ useAudit: () => state.findings }));
vi.mock('../../hooks/useUpdates', () => ({ useUpdates: () => state.updates }));
vi.mock('../../hooks/useSourceHealth', () => ({ useSourceHealth: () => state.health }));
vi.mock('../../hooks/useServerStopped', () => ({
  useServerStopped: () => state.stopped,
  clearServerStopped: vi.fn(),
}));
vi.mock('../../context/TauriContext', () => ({
  useTauri: () => ({ appInfo: { serverRunning: true, serverPort: 19420 } }),
}));
vi.mock('../../context/ProjectContext', () => ({ useProjects: () => ({ reloadView }) }));

function finding(severity: AuditFinding['severity'], skill = 'pdf'): AuditFinding {
  return { skill, kind: 'skill', severity, message: 'issue', file: 'SKILL.md', line: 3 };
}

function LocationProbe() {
  const { pathname, search } = useLocation();
  return <span data-testid="location">{pathname + search}</span>;
}

function renderButton() {
  return render(
    <MemoryRouter>
      <StatusButton />
      <LocationProbe />
    </MemoryRouter>
  );
}

function openPanel() {
  fireEvent.click(screen.getByRole('button', { expanded: false }));
}

afterEach(() => {
  state.findings = [];
  state.updates = { ...state.updates, cli: null, app: null, skills: [], plugins: [] };
  state.health = { outOfSyncTargets: [], git: null };
  state.stopped = false;
  vi.clearAllMocks();
});

describe('StatusButton', () => {
  it('renders nothing when only the running server is left to report', () => {
    renderButton();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('counts everything with an action in a neutral label', () => {
    state.updates = { ...state.updates, skills: ['pdf'] };
    state.health = { ...state.health, outOfSyncTargets: ['claude'] };
    renderButton();
    expect(screen.getByRole('button')).toHaveTextContent('2 things to review');
  });

  it('leads with security issues and counts the rest', () => {
    state.findings = [finding('CRITICAL'), finding('HIGH', 'xlsx'), finding('LOW')];
    state.updates = { ...state.updates, skills: ['pdf'] };
    renderButton();
    expect(screen.getByRole('button')).toHaveTextContent('2 security issues+1');
  });

  it('keeps low and medium findings out of the security label', () => {
    state.findings = [finding('MEDIUM'), finding('LOW')];
    renderButton();
    expect(screen.getByRole('button')).toHaveTextContent('1 thing to review');
  });

  it('orders rows by severity in plain words, with the server last', () => {
    state.findings = [finding('HIGH')];
    state.updates = { ...state.updates, skills: ['pdf'] };
    state.health = {
      outOfSyncTargets: ['claude'],
      git: { uncommitted: 1, ahead: 2, behind: 2 },
    };
    renderButton();
    openPanel();
    expect(screen.getAllByTestId(/^status-row-/).map((row) => row.textContent)).toEqual([
      '1 security issuepdfReview',
      '1 skill updatepdfUpdate',
      '1 target needs a syncclaudeSync',
      '3 changes not pushed1 uncommitted change, 2 commitsPush',
      '2 updates to pull2 commits on the remotePull',
      'Server runningPort 19420',
    ]);
  });

  it('lists new app and CLI versions after security issues', () => {
    state.findings = [finding('HIGH')];
    state.updates = { ...state.updates, app: '0.6.0', cli: 'v0.24.0', skills: ['pdf'] };
    renderButton();
    openPanel();
    expect(screen.getAllByTestId(/^status-row-/).map((row) => row.textContent)).toEqual([
      '1 security issuepdfReview',
      'Skillshare App v0.6.0 availableA new version is ready to installUpdate',
      'skillshare CLI v0.24.0 availableA new version is ready to installUpdate',
      '1 skill updatepdfUpdate',
      'Server runningPort 19420',
    ]);
  });

  it('opens the CLI settings tab to install a new CLI', () => {
    state.updates = { ...state.updates, cli: 'v0.24.0' };
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/settings?tab=cli');
  });

  it('closes the panel on Escape', () => {
    state.updates = { ...state.updates, plugins: ['hud'] };
    renderButton();
    openPanel();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Status' })).not.toBeInTheDocument();
  });

  it('opens the update page instead of updating, and closes the panel', () => {
    state.updates = { ...state.updates, skills: ['pdf'] };
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/skills?tab=updates');
    expect(screen.queryByRole('dialog', { name: 'Status' })).not.toBeInTheDocument();
  });

  it('lists skill and plugin updates as separate rows', () => {
    state.updates = { ...state.updates, skills: ['archify'], plugins: ['ponytail'] };
    renderButton();
    openPanel();
    expect(screen.getAllByTestId(/^status-row-/).map((row) => row.textContent)).toEqual([
      '1 skill updatearchifyUpdate',
      '1 plugin updateponytailUpdate',
      'Server runningPort 19420',
    ]);
  });

  it('opens the plugins page for plugin updates', () => {
    state.updates = { ...state.updates, plugins: ['ponytail'] };
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/plugins');
  });

  it('opens the git page for changes to pull', () => {
    state.health = { ...state.health, git: { uncommitted: 0, ahead: 0, behind: 1 } };
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Pull' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/git');
  });

  it('opens the skill a security issue is in', () => {
    state.findings = [finding('HIGH', 'frontend/doctor')];
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/skills/frontend%2Fdoctor');
  });

  it('opens the audit page for issues in several skills', () => {
    state.findings = [finding('HIGH', 'pdf'), finding('HIGH', 'xlsx')];
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/audit');
  });

  it('restarts a stopped server through the header reload', () => {
    state.stopped = true;
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Restart' }));
    expect(reloadView).toHaveBeenCalled();
  });

  it('re-checks everything on Check now', () => {
    state.updates = { ...state.updates, skills: ['pdf'] };
    renderButton();
    openPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Check now' }));
    expect(bridge.checkStatusNow).toHaveBeenCalled();
  });
});
