import {
  INSTALL_STEPS,
  installStepIndex,
  isInstallStopped,
  type FailableStep,
  type InstallPhase,
  type InstallStep,
} from '../../hooks/useAppUpdate';
import type { CliUpgradeStatus } from '../../hooks/useCliUpgrade';

export type StepState = 'done' | 'running' | 'pending' | 'failed';
export interface StepView {
  key: string;
  state: StepState;
  title: string;
  sub?: string;
  /** Right-aligned value, e.g. the download percentage. */
  right?: string;
  /** Width (0-100) of the thin line under a running step that reports progress. */
  line?: number;
}

export type HeroTone = 'busy' | 'ok' | 'warn';
/** The button the progress card offers: restart the app, or retry the app or CLI update. */
export type HeroAction = 'restart-app' | 'retry-app' | 'retry-cli';
export interface Hero {
  tone: HeroTone;
  title: string;
  label?: string;
  sub?: string;
  action?: HeroAction;
}

/** The part of the CLI upgrade store the progress card reads. */
export interface CliRun {
  status: CliUpgradeStatus;
  step: string | null;
  startedAt: number;
  newVersion: string | null;
  error: string | null;
  withApp: boolean;
}

/** The part of the app update store the progress card reads. */
export interface AppRun {
  phase: InstallPhase;
  failedPhase: FailableStep | null;
  downloaded: number;
  total: number;
  phaseAt: number;
  installError: string | null;
}

export interface UpdateRun {
  cli: CliRun;
  app: AppRun;
  /** The CLI version being installed, or the newest known one. */
  cliTarget: string | null;
  /** The app version being installed, if one is available. */
  appTarget: string | null;
  now: number;
}

export interface UpdateProgress {
  steps: StepView[];
  /** 0-100: finished steps plus the download's share. */
  overall: number;
  hero: Hero;
}

export const formatVersion = (version: string) => `v${version.replace(/^v/, '')}`;
const toMegabytes = (bytes: number) => (bytes / 1048576).toFixed(1);

export function elapsed(since: number, now: number) {
  const totalSeconds = Math.max(0, Math.floor((now - since) / 1000));
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, '0')}`;
}

/** Text and numbers both the steps and the hero show. */
interface Shared {
  cliLine: string;
  appLabel: string;
  cliLabel: string;
  downloadPercent: number;
}

/** The steps of the update that runs, from the start: the CLI, then the app. */
function listSteps({ cli, app, now }: UpdateRun, { cliLine, appLabel, downloadPercent }: Shared) {
  const cliRunning = cli.status === 'upgrading';
  const cliInPlan = cli.status !== 'idle';
  const appInPlan = app.phase !== 'idle' || (cli.withApp && (cliRunning || cli.status === 'error'));

  const currentInstallStep =
    app.phase === 'restart-failed'
      ? INSTALL_STEPS.indexOf('restarting')
      : app.phase === 'error'
        ? INSTALL_STEPS.indexOf(app.failedPhase ?? 'downloading')
        : installStepIndex(app.phase);
  const installStepState = (step: InstallStep): StepState => {
    const at = INSTALL_STEPS.indexOf(step);
    if (at < currentInstallStep) return 'done';
    if (at > currentInstallStep) return 'pending';
    return isInstallStopped(app.phase) ? 'failed' : 'running';
  };

  const steps: StepView[] = [];
  if (cliInPlan) {
    const state: StepState = cliRunning ? 'running' : cli.status === 'error' ? 'failed' : 'done';
    steps.push({
      key: 'cli',
      state,
      title:
        state === 'running'
          ? 'Updating the CLI'
          : state === 'failed'
            ? 'CLI update failed'
            : cli.status === 'up-to-date'
              ? 'The CLI is already on the latest version'
              : `CLI updated to ${cli.newVersion ? formatVersion(cli.newVersion) : 'the latest version'}`,
      sub:
        state === 'running'
          ? cliLine
          : cli.status === 'done'
            ? 'The server restarted on the new version.'
            : undefined,
    });
  }
  if (appInPlan) {
    const download = installStepState('downloading');
    const install = installStepState('installing');
    const restart = installStepState('restarting');
    steps.push(
      {
        key: 'download',
        state: download,
        title: `Download ${appLabel}`,
        sub:
          download === 'running'
            ? app.total > 0
              ? `${toMegabytes(app.downloaded)} of ${toMegabytes(app.total)} MB`
              : `${toMegabytes(app.downloaded)} MB`
            : undefined,
        right: download === 'running' && app.total > 0 ? `${downloadPercent}%` : undefined,
        line: download === 'running' ? downloadPercent : undefined,
      },
      {
        key: 'install',
        state: install,
        title: 'Install',
        sub: install === 'running' ? `Installing... ${elapsed(app.phaseAt, now)}` : undefined,
      },
      {
        key: 'restart',
        state: restart,
        title: restart === 'failed' ? 'Restart to finish' : 'Restart',
        sub: restart === 'running' ? `Restarting... ${elapsed(app.phaseAt, now)}` : undefined,
      }
    );
  }
  return steps;
}

/** The banner above the steps: what runs now, what failed, or how the run ended. */
function pickHero(
  { cli, app }: UpdateRun,
  steps: StepView[],
  { cliLine, appLabel, cliLabel }: Shared
): Hero {
  const running = steps.findIndex((s) => s.state === 'running');

  if (app.phase === 'restart-failed')
    return {
      tone: 'warn',
      title: 'Restart to finish',
      sub: 'The update is installed but the app could not restart itself.',
      action: 'restart-app',
    };
  if (app.phase === 'error')
    return {
      tone: 'warn',
      title: 'App update failed',
      label: appLabel,
      sub: app.installError ?? '',
      action: 'retry-app',
    };
  if (cli.status === 'error')
    return {
      tone: 'warn',
      title: 'CLI update failed',
      label: cliLabel,
      sub: cli.error ?? '',
      action: 'retry-cli',
    };
  if (running >= 0) {
    const onCli = steps[running].key === 'cli';
    return {
      tone: 'busy',
      title: steps.length > 1 ? `Updating ${running + 1} of ${steps.length}` : 'Updating',
      label: onCli ? cliLabel : appLabel,
      sub: !onCli
        ? 'The app restarts by itself and reopens. You can keep working until then.'
        : steps.length > 1
          ? 'The CLI updates first. Then the app installs its update and restarts.'
          : cliLine,
    };
  }
  return cli.status === 'done'
    ? {
        tone: 'ok',
        title: 'CLI updated',
        label: cli.newVersion ? `skillshare CLI ${formatVersion(cli.newVersion)}` : undefined,
        sub: 'The server restarted on the new version.',
      }
    : { tone: 'ok', title: 'The CLI is already on the latest version' };
}

/** Everything the progress card shows, or null when no update has started. */
export function updateProgress(run: UpdateRun): UpdateProgress | null {
  const { cli, app, appTarget, cliTarget, now } = run;
  const downloadPercent =
    app.total > 0 ? Math.min(100, Math.round((app.downloaded / app.total) * 100)) : 0;
  const shared: Shared = {
    cliLine: `${cli.step ?? 'Starting…'} ${elapsed(cli.startedAt, now)}`,
    appLabel: appTarget ? `skillshare App ${formatVersion(appTarget)}` : 'skillshare App',
    cliLabel: cliTarget ? `skillshare CLI ${formatVersion(cliTarget)}` : 'skillshare CLI',
    downloadPercent,
  };
  const steps = listSteps(run, shared);
  if (steps.length === 0) return null;

  const done = steps.filter((s) => s.state === 'done').length;
  const downloadShare = app.phase === 'downloading' ? downloadPercent / 100 : 0;
  return {
    steps,
    overall: Math.round(((done + downloadShare) / steps.length) * 100),
    hero: pickHero(run, steps, shared),
  };
}
