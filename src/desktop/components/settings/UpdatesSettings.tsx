import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  AppWindow,
  Check,
  CircleAlert,
  Download,
  LoaderCircle,
  RefreshCw,
  Terminal,
} from 'lucide-react';
import Button from '../../../components/Button';
import Switch from '../../../components/Switch';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';
import { clearAppUpdateResult, useAppUpdate } from '../../hooks/useAppUpdate';
import { clearCliUpgrade, upgradeCli, useCliUpgrade } from '../../hooks/useCliUpgrade';
import { checkResourcesNow, checkVersionsNow, useUpdates } from '../../hooks/useUpdates';
import {
  SettingsHeader,
  SettingsList,
  SettingsRow,
  SettingsSection,
  StatusDot,
} from './SettingsLayout';

const APP_REPO = 'https://github.com/runkids/skillshare-app';
const CLI_REPO = 'https://github.com/runkids/skillshare';
const COULD_NOT_CHECK = 'Couldn’t reach github.com. Check your connection and try again.';
const v = (version: string) => `v${version.replace(/^v/, '')}`;
const mb = (bytes: number) => (bytes / 1048576).toFixed(1);

function elapsed(since: number, now: number) {
  const s = Math.max(0, Math.floor((now - since) / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** The webview ignores target=_blank, so hand external links to the system browser. */
function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
      className="ml-2 font-sans text-[13px] text-[var(--accent)] hover:underline"
    >
      {children}
    </a>
  );
}

function Tile({ children }: { children: ReactNode }) {
  return (
    <span
      className="w-8 h-8 shrink-0 flex items-center justify-center rounded-[var(--r-ctl)] bg-[var(--sunken)] text-[var(--ink-2)]"
      aria-hidden="true"
    >
      {children}
    </span>
  );
}

/** The 40px icon that leads the banner and the progress card. */
function HeroIcon({ tone }: { tone: 'busy' | 'ok' | 'warn' }) {
  const look = {
    busy: 'bg-[var(--sunken)] text-[var(--ink)]',
    ok: 'ss-pop bg-[var(--ok-bg)] text-[var(--ok)]',
    warn: 'bg-[var(--warn-bg)] text-[var(--warn)]',
  }[tone];
  return (
    <span
      className={`w-10 h-10 shrink-0 flex items-center justify-center rounded-[11px] ${look}`}
      aria-hidden="true"
    >
      {tone === 'busy' && <Download size={20} strokeWidth={1.75} />}
      {tone === 'ok' && <Check size={20} strokeWidth={2.6} />}
      {tone === 'warn' && <CircleAlert size={20} strokeWidth={1.75} />}
    </span>
  );
}

/** A quiet Check in a section heading; the icon spins while the check runs. */
function CheckButton({
  label,
  checking,
  disabled,
  onClick,
}: {
  label: string;
  checking: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-busy={checking}
      onClick={onClick}
      disabled={disabled || checking}
      className={`inline-flex items-center gap-1.5 h-7 px-2.5 -mr-2.5 rounded-[var(--r-ctl)] text-[13px] font-medium whitespace-nowrap text-[var(--ink-2)] enabled:hover:text-[var(--ink)] enabled:cursor-pointer ${disabled && !checking ? 'opacity-50' : ''}`}
    >
      <RefreshCw
        size={14}
        strokeWidth={1.75}
        className={checking ? 'ss-spin' : undefined}
        aria-hidden="true"
      />
      {checking ? 'Checking…' : 'Check'}
    </button>
  );
}

type StepState = 'done' | 'running' | 'pending' | 'failed';
interface StepView {
  key: string;
  state: StepState;
  title: string;
  sub?: string;
  /** Right-aligned value, e.g. the download percentage. */
  right?: string;
  /** Width (0-100) of the thin line under a running step that reports progress. */
  line?: number;
}

function StepMarker({ state }: { state: StepState }) {
  if (state === 'done')
    return (
      <span className="ss-pop w-6 h-6 shrink-0 rounded-full flex items-center justify-center bg-[var(--ok-bg)] text-[var(--ok)]">
        <Check size={14} strokeWidth={2.6} />
      </span>
    );
  if (state === 'running')
    return (
      <span className="w-6 h-6 shrink-0 flex items-center justify-center text-[var(--ink)]">
        <LoaderCircle size={20} strokeWidth={2.25} className="ss-spin" />
      </span>
    );
  if (state === 'failed')
    return (
      <span className="w-6 h-6 shrink-0 rounded-full flex items-center justify-center bg-[var(--warn-bg)] text-[var(--warn)] text-sm font-bold">
        !
      </span>
    );
  return (
    <span className="w-6 h-6 shrink-0 box-border rounded-full border-2 border-[var(--line-2)]" />
  );
}

function Step({ step }: { step: StepView }) {
  return (
    <li className="relative isolate flex items-start gap-3.5 py-3.5">
      {step.state === 'done' && (
        <span
          aria-hidden="true"
          className="ss-flash absolute inset-y-0 -inset-x-5 -z-10 pointer-events-none"
        />
      )}
      <span aria-hidden="true" className="flex shrink-0">
        <StepMarker state={step.state} />
      </span>
      <div className="flex-1 min-w-0 flex flex-col gap-1">
        <span
          className={`text-sm font-semibold ${step.state === 'pending' ? 'text-[var(--ink-2)]' : 'text-[var(--ink)]'}`}
        >
          {step.title}
        </span>
        {step.sub && <span className="text-[13px] text-[var(--ink-2)]">{step.sub}</span>}
      </div>
      {step.right && (
        <span className="font-mono text-[13px] text-[var(--ink-2)] whitespace-nowrap">
          {step.right}
        </span>
      )}
      {step.line !== undefined && (
        <span
          aria-hidden="true"
          className="absolute left-0 -bottom-px h-0.5 bg-[var(--ink)] transition-[width] duration-[450ms] ease-[cubic-bezier(.3,.7,.3,1)]"
          style={{ width: `${step.line}%` }}
        />
      )}
    </li>
  );
}

const APP_PHASES = ['downloading', 'installing', 'restarting'] as const;

export default function UpdatesSettings() {
  const navigate = useNavigate();
  const { appInfo, refresh } = useTauri();
  const { reloadView } = useProjects();
  const updates = useUpdates();
  const app = useAppUpdate();
  const cli = useCliUpgrade();
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [cliCheckFailed, setCliCheckFailed] = useState(false);
  const [resourcesSince, setResourcesSince] = useState<number | null>(null);
  const [resourcesChecked, setResourcesChecked] = useState(0);
  /** Why the last skills check missed some sources; `null` after a full check. */
  const [resourcesError, setResourcesError] = useState<string | null>(null);
  const [notifyUpdate, setNotifyUpdate] = useState(true);
  const [notifyError, setNotifyError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => {});
    tauriBridge
      .getNotifyUpdate()
      .then(setNotifyUpdate)
      .catch(() => {});
    void checkNow();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- check the versions once on entry
  }, []);

  // The progress card is cleared when the user leaves, unless an update is still running.
  const appPhase = useRef(app.phase);
  useEffect(() => {
    appPhase.current = app.phase;
  });
  useEffect(
    () => () => {
      if ((APP_PHASES as readonly string[]).includes(appPhase.current)) return;
      clearAppUpdateResult();
      clearCliUpgrade();
    },
    []
  );

  const cliRunning = cli.status === 'upgrading';
  const appRunning = (APP_PHASES as readonly string[]).includes(app.phase);
  const busy = cliRunning || appRunning;
  const ticking =
    cliRunning || app.phase === 'installing' || app.phase === 'restarting' || !!resourcesSince;
  useEffect(() => {
    if (!ticking) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [ticking]);

  const appNew = app.version;
  const cliNew = cli.status === 'done' || cli.status === 'up-to-date' ? null : updates.cli;
  const appStopped = app.phase === 'error' || app.phase === 'restart-failed';
  const versionsChecking = checking || app.status === 'checking';
  // While the app waits for a manual restart, a check only spins its button.
  const rowsChecking = versionsChecking && !busy && app.phase !== 'restart-failed';
  const idle = !busy && !appStopped && cli.status !== 'error' && !rowsChecking;
  const both = idle && Boolean(appNew && cliNew);

  const runCli = (then?: () => unknown) =>
    upgradeCli({
      oldVersion: appInfo?.cliVersion,
      refresh,
      reloadView: () => reloadView(),
      then,
    });
  const updateBoth = () => runCli(app.install);
  const updateApp = () => {
    clearCliUpgrade();
    void app.install();
  };
  const checkNow = async () => {
    setChecking(true);
    const [, versions] = await Promise.allSettled([app.recheck(), checkVersionsNow()]);
    setCliCheckFailed(versions.status === 'rejected');
    setChecking(false);
  };
  const checkResources = async () => {
    setResourcesSince(Date.now());
    const error = await checkResourcesNow().then(
      () => null,
      (err: unknown) => (err instanceof Error ? err.message : String(err))
    );
    setResourcesSince(null);
    setResourcesError(error);
    if (!error) setResourcesChecked((n) => n + 1);
  };
  const toggleNotify = async (enabled: boolean) => {
    setNotifyUpdate(enabled);
    setNotifyError(null);
    try {
      await tauriBridge.setNotifyUpdate(enabled);
    } catch (err) {
      setNotifyUpdate(!enabled);
      setNotifyError(err instanceof Error ? err.message : String(err));
    }
  };

  const resources = [
    { names: updates.skills, one: 'skill', many: 'skills', path: '/skills?tab=updates' },
    { names: updates.agents, one: 'agent', many: 'agents', path: '/agents?tab=updates' },
    { names: updates.plugins, one: 'plugin', many: 'plugins', path: '/plugins' },
    {
      names: updates.repositories,
      one: 'tracked repository',
      many: 'tracked repositories',
      path: '/skills?tab=updates',
    },
  ].filter((r) => r.names.length > 0);

  // Every step of the update that runs shows from the start: the CLI, then the app.
  const appPct = app.total > 0 ? Math.min(100, Math.round((app.downloaded / app.total) * 100)) : 0;
  const cliLine = `${cli.step ?? 'Starting…'} ${elapsed(cli.startedAt, now)}`;
  const cliTarget = cli.newVersion ?? cliNew ?? updates.cli;
  const appLabel = appNew ? `skillshare App ${v(appNew)}` : 'skillshare App';
  const cliLabel = cliTarget ? `skillshare CLI ${v(cliTarget)}` : 'skillshare CLI';
  const cliInPlan = cli.status !== 'idle';
  const appInPlan = app.phase !== 'idle' || (cli.withApp && (cliRunning || cli.status === 'error'));
  const appAt =
    app.phase === 'restart-failed'
      ? 2
      : app.phase === 'error'
        ? APP_PHASES.indexOf(app.failedPhase ?? 'downloading')
        : (APP_PHASES as readonly string[]).indexOf(app.phase);
  const appStep = (i: number): StepState =>
    i < appAt ? 'done' : i === appAt ? (appStopped ? 'failed' : 'running') : 'pending';

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
              : `CLI updated to ${cli.newVersion ? v(cli.newVersion) : 'the latest version'}`,
      sub:
        state === 'running'
          ? cliLine
          : cli.status === 'done'
            ? 'The server restarted on the new version.'
            : undefined,
    });
  }
  if (appInPlan) {
    const download = appStep(0);
    const install = appStep(1);
    const restart = appStep(2);
    steps.push(
      {
        key: 'download',
        state: download,
        title: `Download ${appLabel}`,
        sub:
          download === 'running'
            ? app.total > 0
              ? `${mb(app.downloaded)} of ${mb(app.total)} MB`
              : `${mb(app.downloaded)} MB`
            : undefined,
        right: download === 'running' && app.total > 0 ? `${appPct}%` : undefined,
        line: download === 'running' ? appPct : undefined,
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
  const total = steps.length;
  const running = steps.findIndex((s) => s.state === 'running');
  const done = steps.filter((s) => s.state === 'done').length;
  // The bar moves only on real progress: finished steps plus the download's share.
  const overall = total
    ? Math.round(((done + (app.phase === 'downloading' ? appPct / 100 : 0)) / total) * 100)
    : 0;

  let hero: { icon: 'busy' | 'ok' | 'warn'; title: string; label?: string; sub?: string };
  let heroAction: ReactNode = null;
  if (app.phase === 'restart-failed') {
    hero = {
      icon: 'warn',
      title: 'Restart to finish',
      sub: 'The update is installed but the app could not restart itself.',
    };
    heroAction = <Button onClick={app.restart}>Restart now</Button>;
  } else if (app.phase === 'error') {
    hero = {
      icon: 'warn',
      title: 'App update failed',
      label: appLabel,
      sub: app.installError ?? '',
    };
    heroAction = <Button onClick={app.install}>Retry</Button>;
  } else if (cli.status === 'error') {
    hero = { icon: 'warn', title: 'CLI update failed', label: cliLabel, sub: cli.error ?? '' };
    heroAction = (
      <Button onClick={() => void (cli.withApp ? updateBoth() : runCli())}>Retry</Button>
    );
  } else if (running >= 0) {
    const onCli = steps[running].key === 'cli';
    hero = {
      icon: 'busy',
      title: total > 1 ? `Updating ${running + 1} of ${total}` : 'Updating',
      label: onCli ? cliLabel : appLabel,
      sub: !onCli
        ? 'The app restarts by itself and reopens. You can keep working until then.'
        : total > 1
          ? 'The CLI updates first. Then the app installs its update and restarts.'
          : cliLine,
    };
  } else {
    hero =
      cli.status === 'done'
        ? {
            icon: 'ok',
            title: 'CLI updated',
            label: cli.newVersion ? `skillshare CLI ${v(cli.newVersion)}` : undefined,
            sub: 'The server restarted on the new version.',
          }
        : { icon: 'ok', title: 'The CLI is already on the latest version' };
  }

  const appCheckFailed = app.status === 'error';
  const appStatus = rowsChecking ? null : appCheckFailed ? (
    <StatusDot tone="warn">Couldn’t check</StatusDot>
  ) : !appNew ? (
    <StatusDot tone="ok">Up to date</StatusDot>
  ) : null;
  const cliStatus = rowsChecking ? null : cliCheckFailed ? (
    <StatusDot tone="warn">Couldn’t check</StatusDot>
  ) : !cliNew ? (
    <StatusDot tone="ok">Up to date</StatusDot>
  ) : null;

  const resourceRows = resources.map((r) => (
    <SettingsRow
      key={r.one}
      label={`${r.names.length} ${r.names.length === 1 ? r.one : r.many}`}
      hint={<span className="font-mono">{r.names.join(', ')}</span>}
    >
      <Button size="sm" variant="secondary" onClick={() => navigate(r.path)}>
        Review
      </Button>
    </SettingsRow>
  ));

  return (
    <div className="flex flex-col gap-7">
      <SettingsHeader title="Updates" />

      {both && (
        <section
          aria-label="Update the app and the CLI"
          className="flex flex-wrap items-center gap-4 px-5 py-[18px] rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)]"
          style={{ border: 'var(--frame)' }}
        >
          <HeroIcon tone="busy" />
          <div className="flex-1 basis-60 min-w-0 flex flex-col gap-0.5">
            <span style={{ font: 'var(--h2)', letterSpacing: 'var(--h2-track)' }}>
              New versions of the app and the CLI
            </span>
            <span className="text-[13px] text-[var(--ink-2)]">
              The CLI updates first. Then the app installs its update and restarts.
            </span>
          </div>
          <Button onClick={updateBoth}>Update both</Button>
        </section>
      )}

      {total > 0 && (
        <section
          aria-label="Update progress"
          aria-live="polite"
          className="flex flex-col gap-4 px-5 py-[18px] overflow-hidden rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)]"
          style={{ border: 'var(--frame)' }}
        >
          <div className="flex flex-wrap items-center gap-4">
            <HeroIcon tone={hero.icon} />
            <div className="flex-1 basis-60 min-w-0 flex flex-col gap-0.5">
              <div className="text-base font-semibold text-[var(--ink)]">
                {hero.title}
                {hero.label && (
                  <span className="ml-3 font-mono text-sm font-medium text-[var(--ink-2)]">
                    {hero.label}
                  </span>
                )}
              </div>
              {hero.sub && <div className="text-[13px] text-[var(--ink-2)]">{hero.sub}</div>}
            </div>
            {heroAction}
          </div>
          <div
            role="progressbar"
            aria-label="Update progress"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={overall}
            className="relative h-1.5 rounded-full bg-[var(--sunken)] overflow-hidden"
          >
            <span
              className="block h-full rounded-[inherit] bg-[var(--ink)] transition-[width] duration-[450ms] ease-[cubic-bezier(.3,.7,.3,1)]"
              style={{ width: `${overall}%` }}
            />
          </div>
          {total > 1 && (
            <ol className="m-0 p-0 list-none [border-top:var(--sep)] [&>*+*]:[border-top:var(--sep)]">
              {steps.map((step) => (
                <Step key={step.key} step={step} />
              ))}
            </ol>
          )}
        </section>
      )}

      <SettingsSection
        title="Versions"
        heading="h2"
        action={
          <CheckButton
            label="Check versions"
            checking={versionsChecking}
            disabled={busy}
            onClick={() => {
              // "Restart to finish" keeps its card, CLI step included; only a finished run clears.
              if (app.phase !== 'restart-failed') clearCliUpgrade();
              void checkNow();
            }}
          />
        }
      >
        <SettingsList>
          <div className="flex flex-wrap items-center gap-3.5 p-4">
            <Tile>
              <AppWindow size={16} />
            </Tile>
            <div className="flex-1 basis-[200px] min-w-0 flex flex-col gap-0.5">
              <span className="text-sm font-semibold">skillshare App</span>
              <span className="font-mono text-[13px] text-[var(--ink-2)]">
                {appVersion ? v(appVersion) : '…'}
                {appNew && !rowsChecking && (
                  <>
                    {` → ${v(appNew)} `}
                    <ExternalLink href={`${APP_REPO}/releases/tag/${v(appNew)}`}>
                      Release notes
                    </ExternalLink>
                  </>
                )}
              </span>
              {appCheckFailed && !rowsChecking && (
                <span className="text-[13px] text-[var(--ink-2)]" title={app.error ?? undefined}>
                  {COULD_NOT_CHECK}
                </span>
              )}
            </div>
            {idle && appNew ? (
              <Button size="sm" variant={both ? 'secondary' : 'primary'} onClick={updateApp}>
                Update and restart
              </Button>
            ) : (
              appStatus
            )}
          </div>

          <div className="flex flex-wrap items-center gap-3.5 p-4">
            <Tile>
              <Terminal size={16} />
            </Tile>
            <div className="flex-1 basis-[200px] min-w-0 flex flex-col gap-0.5">
              <span className="flex items-center gap-2 text-sm font-semibold">
                skillshare CLI
                {appInfo?.cliSource && (
                  <span className="font-mono text-[11px] font-medium px-1.5 py-0.5 rounded-[var(--r-tag)] bg-[var(--sunken)] text-[var(--ink-2)]">
                    {appInfo.cliSource}
                  </span>
                )}
              </span>
              <span className="font-mono text-[13px] text-[var(--ink-2)]">
                {appInfo?.cliVersion ? v(appInfo.cliVersion) : 'Unknown'}
                {cliNew && !rowsChecking && (
                  <>
                    {` → ${v(cliNew)} `}
                    <ExternalLink href={`${CLI_REPO}/releases/tag/${v(cliNew)}`}>
                      Release notes
                    </ExternalLink>
                  </>
                )}
              </span>
              {cliCheckFailed && !rowsChecking && (
                <span className="text-[13px] text-[var(--ink-2)]">{COULD_NOT_CHECK}</span>
              )}
            </div>
            {idle && cliNew ? (
              <Button
                size="sm"
                variant={both ? 'secondary' : 'primary'}
                onClick={() => void runCli()}
              >
                Update CLI
              </Button>
            ) : (
              cliStatus
            )}
          </div>
        </SettingsList>
      </SettingsSection>

      <SettingsSection
        title="Skills, agents and plugins"
        heading="h2"
        action={
          <CheckButton
            label="Check skills, agents and plugins"
            checking={resourcesSince !== null}
            onClick={() => void checkResources()}
          />
        }
      >
        <div aria-live="polite">
          <SettingsList className="relative isolate overflow-hidden">
            {resourcesSince !== null ? (
              <div className="flex items-center gap-3.5 min-h-14 px-4 py-3">
                <span
                  aria-hidden="true"
                  className="w-6 h-6 shrink-0 flex items-center justify-center text-[var(--ink)]"
                >
                  <LoaderCircle size={20} strokeWidth={2.25} className="ss-spin" />
                </span>
                <div className="flex-1 min-w-0 flex flex-col gap-0.5">
                  <span className="text-sm font-semibold text-[var(--ink)]">
                    Checking skills, agents and plugins
                  </span>
                  <span className="text-[13px] text-[var(--ink-2)]">
                    {`Checking every source... ${elapsed(resourcesSince, now)}`}
                  </span>
                </div>
              </div>
            ) : resourcesError !== null ? (
              <>
                <SettingsRow
                  label="Couldn’t check every source"
                  hint={
                    <span title={resourcesError}>
                      Some sources didn’t answer. Check your connection and try again.
                    </span>
                  }
                />
                {resourceRows}
              </>
            ) : resources.length === 0 ? (
              <SettingsRow
                label="No updates"
                hint="Installed skills, agents and plugins match their sources."
              />
            ) : (
              resourceRows
            )}
            {resourcesChecked > 0 && resourcesSince === null && (
              <span
                key={resourcesChecked}
                aria-hidden="true"
                className="ss-flash absolute inset-0 -z-10 pointer-events-none"
                style={{ borderTop: 0 }}
              />
            )}
          </SettingsList>
        </div>
      </SettingsSection>

      <SettingsList>
        <SettingsRow
          id="notify-update"
          label="Tell me about new versions"
          hint={
            notifyError ??
            'Sends a notification about new app and CLI versions while the app is in the background.'
          }
        >
          <Switch
            checked={notifyUpdate}
            onChange={toggleNotify}
            labelledBy="notify-update"
            describedBy="notify-update-hint"
          />
        </SettingsRow>
      </SettingsList>
    </div>
  );
}
