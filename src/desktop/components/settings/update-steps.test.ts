import { describe, expect, it } from 'vitest';
import {
  elapsed,
  formatVersion,
  updateProgress,
  type AppRun,
  type CliRun,
  type UpdateRun,
} from './update-steps';

const IDLE_CLI: CliRun = {
  status: 'idle',
  step: null,
  startedAt: 0,
  newVersion: null,
  error: null,
  withApp: false,
};
const IDLE_APP: AppRun = {
  phase: 'idle',
  failedPhase: null,
  downloaded: 0,
  total: 0,
  phaseAt: 0,
  installError: null,
};

function run(cli: Partial<CliRun> = {}, app: Partial<AppRun> = {}): UpdateRun {
  return {
    cli: { ...IDLE_CLI, ...cli },
    app: { ...IDLE_APP, ...app },
    cliTarget: '0.24.1',
    appTarget: '0.9.0',
    now: 65_000,
  };
}

const states = (r: UpdateRun) => updateProgress(r)?.steps.map((s) => `${s.key}:${s.state}`);

describe('update steps', () => {
  it('shows nothing before an update starts', () => {
    expect(updateProgress(run())).toBeNull();
  });

  it('lists the CLI step alone for a CLI-only upgrade', () => {
    const progress = updateProgress(run({ status: 'upgrading', step: 'Downloading v0.24.1...' }));
    expect(progress?.steps).toEqual([
      {
        key: 'cli',
        state: 'running',
        title: 'Updating the CLI',
        sub: 'Downloading v0.24.1... 1:05',
      },
    ]);
    expect(progress?.hero).toMatchObject({
      tone: 'busy',
      title: 'Updating',
      sub: 'Downloading v0.24.1... 1:05',
    });
  });

  it('lists the CLI and every app step from the start of Update both', () => {
    const r = run({ status: 'upgrading', withApp: true });
    expect(states(r)).toEqual([
      'cli:running',
      'download:pending',
      'install:pending',
      'restart:pending',
    ]);
    expect(updateProgress(r)?.hero).toMatchObject({
      title: 'Updating 1 of 4',
      label: 'skillshare CLI v0.24.1',
    });
  });

  it('moves the bar by finished steps plus the share of the download', () => {
    const r = run({ status: 'done' }, { phase: 'downloading', downloaded: 5, total: 10 });
    expect(states(r)).toEqual([
      'cli:done',
      'download:running',
      'install:pending',
      'restart:pending',
    ]);
    expect(updateProgress(r)?.overall).toBe(38);
    expect(updateProgress(r)?.steps[1]).toMatchObject({ right: '50%', line: 50 });
  });

  it('shows the size without a percentage when the download size is unknown', () => {
    const progress = updateProgress(run({}, { phase: 'downloading', downloaded: 2_097_152 }));
    expect(progress?.steps[0]).toMatchObject({ sub: '2.0 MB', right: undefined });
  });

  it('shows elapsed time on the install and restart steps', () => {
    const installing = updateProgress(run({}, { phase: 'installing', phaseAt: 5_000 }));
    expect(installing?.steps[1].sub).toBe('Installing... 1:00');
    const restarting = updateProgress(run({}, { phase: 'restarting', phaseAt: 35_000 }));
    expect(restarting?.steps[2].sub).toBe('Restarting... 0:30');
  });

  it('fails the step that was running when the install broke and offers a retry', () => {
    const progress = updateProgress(
      run({}, { phase: 'error', failedPhase: 'installing', installError: 'disk full' })
    );
    expect(progress?.steps.map((s) => s.state)).toEqual(['done', 'failed', 'pending']);
    expect(progress?.hero).toMatchObject({
      tone: 'warn',
      title: 'App update failed',
      sub: 'disk full',
      action: 'retry-app',
    });
  });

  it('fails the restart step when the app could not restart itself', () => {
    const progress = updateProgress(run({}, { phase: 'restart-failed' }));
    expect(progress?.steps.map((s) => s.state)).toEqual(['done', 'done', 'failed']);
    expect(progress?.steps[2].title).toBe('Restart to finish');
    expect(progress?.hero).toMatchObject({ tone: 'warn', action: 'restart-app' });
  });

  it('fails the CLI step and offers a retry when the CLI upgrade errors', () => {
    const progress = updateProgress(run({ status: 'error', error: 'boom', withApp: true }));
    expect(progress?.steps.map((s) => s.state)).toEqual([
      'failed',
      'pending',
      'pending',
      'pending',
    ]);
    expect(progress?.hero).toMatchObject({ tone: 'warn', sub: 'boom', action: 'retry-cli' });
  });

  it('ends a CLI-only upgrade with a success banner naming the new version', () => {
    const progress = updateProgress(run({ status: 'done', newVersion: '0.24.1' }));
    expect(progress?.hero).toMatchObject({
      tone: 'ok',
      title: 'CLI updated',
      label: 'skillshare CLI v0.24.1',
    });
    expect(progress?.overall).toBe(100);
  });

  it('says so when the CLI was already on the latest version', () => {
    const progress = updateProgress(run({ status: 'up-to-date' }));
    expect(progress?.steps[0].title).toBe('The CLI is already on the latest version');
    expect(progress?.hero).toMatchObject({
      tone: 'ok',
      title: 'The CLI is already on the latest version',
    });
  });
});

describe('update formatting', () => {
  it('prefixes a single v to versions', () => {
    expect([formatVersion('1.2.3'), formatVersion('v1.2.3')]).toEqual(['v1.2.3', 'v1.2.3']);
  });

  it('formats elapsed time as minutes and padded seconds, never negative', () => {
    expect([elapsed(0, 65_000), elapsed(10_000, 0)]).toEqual(['1:05', '0:00']);
  });
});
