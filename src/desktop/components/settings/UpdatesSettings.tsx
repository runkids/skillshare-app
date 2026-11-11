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
import {
  clearAppUpdateResult,
  isInstallStep,
  isInstallStopped,
  useAppUpdate,
} from '../../hooks/useAppUpdate';
import { clearCliUpgrade, upgradeCli, useCliUpgrade } from '../../hooks/useCliUpgrade';
import { checkResourcesNow, checkVersionsNow, useVersionResults } from '../../hooks/useUpdates';
import { setUpdatePolicy, useUpdatePolicy } from '../../hooks/useUpdatePolicy';
import {
  SettingsHeader,
  SettingsList,
  SettingsRow,
  SettingsSection,
  StatusDot,
} from './SettingsLayout';
import {
  elapsed,
  formatVersion,
  updateProgress,
  type HeroAction,
  type HeroTone,
  type StepState,
  type StepView,
} from './update-steps';

const APP_REPO = 'https://github.com/runkids/skillshare-app';
const CLI_REPO = 'https://github.com/runkids/skillshare';
const COULD_NOT_CHECK = 'Couldn’t reach github.com. Check your connection and try again.';

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

const HERO_TONES: Record<HeroTone, { look: string; icon: ReactNode }> = {
  busy: {
    look: 'bg-[var(--sunken)] text-[var(--ink)]',
    icon: <Download size={20} strokeWidth={1.75} />,
  },
  ok: {
    look: 'ss-pop bg-[var(--ok-bg)] text-[var(--ok)]',
    icon: <Check size={20} strokeWidth={2.6} />,
  },
  warn: {
    look: 'bg-[var(--warn-bg)] text-[var(--warn)]',
    icon: <CircleAlert size={20} strokeWidth={1.75} />,
  },
};

/** The 40px icon that leads the banner and the progress card. */
function HeroIcon({ tone }: { tone: HeroTone }) {
  const { look, icon } = HERO_TONES[tone];
  return (
    <span
      className={`w-10 h-10 shrink-0 flex items-center justify-center rounded-[11px] ${look}`}
      aria-hidden="true"
    >
      {icon}
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

/** One line of the Versions list: the app or the CLI, its version, and what to do about it. */
function VersionRow({
  icon,
  name,
  badge,
  current,
  available,
  releasesUrl,
  rowsStale,
  checkFailed,
  failureTitle,
  action,
}: {
  icon: ReactNode;
  name: string;
  badge?: string | null;
  current: string;
  /** The newer version, if there is one. */
  available: string | null;
  releasesUrl: string;
  /** A check is replacing what the row shows. */
  rowsStale: boolean;
  checkFailed: boolean;
  failureTitle?: string;
  /** The update button; without one the row shows its status. */
  action: ReactNode;
}) {
  const status = rowsStale ? null : checkFailed ? (
    <StatusDot tone="warn">Couldn’t check</StatusDot>
  ) : !available ? (
    <StatusDot tone="ok">Up to date</StatusDot>
  ) : null;
  return (
    <div className="flex flex-wrap items-center gap-3.5 p-4">
      <Tile>{icon}</Tile>
      <div className="flex-1 basis-[200px] min-w-0 flex flex-col gap-0.5">
        <span className="flex items-center gap-2 text-sm font-semibold">
          {name}
          {badge && (
            <span className="font-mono text-[11px] font-medium px-1.5 py-0.5 rounded-[var(--r-tag)] bg-[var(--sunken)] text-[var(--ink-2)]">
              {badge}
            </span>
          )}
        </span>
        <span className="font-mono text-[13px] text-[var(--ink-2)]">
          {current}
          {available && !rowsStale && (
            <>
              {` → ${formatVersion(available)} `}
              <ExternalLink href={`${releasesUrl}/releases/tag/${formatVersion(available)}`}>
                Release notes
              </ExternalLink>
            </>
          )}
        </span>
        {checkFailed && !rowsStale && (
          <span className="text-[13px] text-[var(--ink-2)]" title={failureTitle}>
            {COULD_NOT_CHECK}
          </span>
        )}
      </div>
      {action ?? status}
    </div>
  );
}

export default function UpdatesSettings() {
  const navigate = useNavigate();
  const { appInfo, refresh } = useTauri();
  const { reloadView } = useProjects();
  const updates = useVersionResults();
  const policy = useUpdatePolicy();
  const [policyError, setPolicyError] = useState<string | null>(null);
  const [policySaving, setPolicySaving] = useState(false);
  const app = useAppUpdate();
  const cli = useCliUpgrade();
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [checkRunning, setCheckRunning] = useState(false);
  const [cliCheckFailed, setCliCheckFailed] = useState(false);
  const [resourcesCheckStartedAt, setResourcesCheckStartedAt] = useState<number | null>(null);
  const [resourcesCheckCount, setResourcesCheckCount] = useState(0);
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
  }, []);

  // The progress card is cleared when the user leaves, unless an update is still running.
  const latestAppPhase = useRef(app.phase);
  useEffect(() => {
    latestAppPhase.current = app.phase;
  });
  useEffect(
    () => () => {
      if (isInstallStep(latestAppPhase.current)) return;
      clearAppUpdateResult();
      clearCliUpgrade();
    },
    []
  );

  const cliRunning = cli.status === 'upgrading';
  const appRunning = isInstallStep(app.phase);
  const busy = cliRunning || appRunning;
  const ticking =
    cliRunning ||
    app.phase === 'installing' ||
    app.phase === 'restarting' ||
    !!resourcesCheckStartedAt;
  useEffect(() => {
    if (!ticking) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [ticking]);

  const appAvailable = app.version;
  const cliAvailable = cli.status === 'done' || cli.status === 'up-to-date' ? null : updates.cli;
  const appStopped = isInstallStopped(app.phase);
  const versionsChecking = !policy || checkRunning || app.status === 'checking';
  // While the app waits for a manual restart, a check only spins its button.
  const rowsStale = versionsChecking && !busy && app.phase !== 'restart-failed';
  const canStartUpdate = !busy && !appStopped && cli.status !== 'error' && !rowsStale;
  const bothAvailable = canStartUpdate && Boolean(appAvailable && cliAvailable);

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
  const checkVersions = async () => {
    setCheckRunning(true);
    const [, versions] = await Promise.allSettled([app.recheck(), checkVersionsNow()]);
    setCliCheckFailed(versions.status === 'rejected');
    setCheckRunning(false);
  };
  const checkedOnEntry = useRef(false);
  useEffect(() => {
    if (checkedOnEntry.current || !policy?.enabled) return;
    checkedOnEntry.current = true;
    let cancelled = false;
    void checkVersionsNow(true).catch(() => {
      if (!cancelled) setCliCheckFailed(true);
    });
    return () => {
      cancelled = true;
    };
  }, [policy?.enabled]);

  const checkResources = async () => {
    setResourcesCheckStartedAt(Date.now());
    const error = await checkResourcesNow().then(
      () => null,
      (err: unknown) => (err instanceof Error ? err.message : String(err))
    );
    setResourcesCheckStartedAt(null);
    setResourcesError(error);
    if (!error) setResourcesCheckCount((n) => n + 1);
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

  const progress = updateProgress({
    cli,
    app,
    appTarget: appAvailable,
    cliTarget: cli.newVersion ?? cliAvailable ?? updates.cli,
    now,
  });
  const heroButtons: Record<HeroAction, ReactNode> = {
    'restart-app': <Button onClick={app.restart}>Restart now</Button>,
    'retry-app': <Button onClick={app.install}>Retry</Button>,
    'retry-cli': (
      <Button onClick={() => void (cli.withApp ? updateBoth() : runCli())}>Retry</Button>
    ),
  };

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

      {bothAvailable && (
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

      {progress && (
        <section
          aria-label="Update progress"
          aria-live="polite"
          className="flex flex-col gap-4 px-5 py-[18px] overflow-hidden rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)]"
          style={{ border: 'var(--frame)' }}
        >
          <div className="flex flex-wrap items-center gap-4">
            <HeroIcon tone={progress.hero.tone} />
            <div className="flex-1 basis-60 min-w-0 flex flex-col gap-0.5">
              <div className="text-base font-semibold text-[var(--ink)]">
                {progress.hero.title}
                {progress.hero.label && (
                  <span className="ml-3 font-mono text-sm font-medium text-[var(--ink-2)]">
                    {progress.hero.label}
                  </span>
                )}
              </div>
              {progress.hero.sub && (
                <div className="text-[13px] text-[var(--ink-2)]">{progress.hero.sub}</div>
              )}
            </div>
            {progress.hero.action && heroButtons[progress.hero.action]}
          </div>
          <div
            role="progressbar"
            aria-label="Update progress"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={progress.overall}
            className="relative h-1.5 rounded-full bg-[var(--sunken)] overflow-hidden"
          >
            <span
              className="block h-full rounded-[inherit] bg-[var(--ink)] transition-[width] duration-[450ms] ease-[cubic-bezier(.3,.7,.3,1)]"
              style={{ width: `${progress.overall}%` }}
            />
          </div>
          {progress.steps.length > 1 && (
            <ol className="m-0 p-0 list-none [border-top:var(--sep)] [&>*+*]:[border-top:var(--sep)]">
              {progress.steps.map((step) => (
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
              void checkVersions();
            }}
          />
        }
      >
        <SettingsList>
          <VersionRow
            icon={<AppWindow size={16} />}
            name="skillshare App"
            current={appVersion ? formatVersion(appVersion) : '…'}
            available={appAvailable}
            releasesUrl={APP_REPO}
            rowsStale={rowsStale}
            checkFailed={app.status === 'error'}
            failureTitle={app.error ?? undefined}
            action={
              canStartUpdate && appAvailable ? (
                <Button
                  size="sm"
                  variant={bothAvailable ? 'secondary' : 'primary'}
                  onClick={updateApp}
                >
                  Update and restart
                </Button>
              ) : null
            }
          />
          <VersionRow
            icon={<Terminal size={16} />}
            name="skillshare CLI"
            badge={appInfo?.cliSource}
            current={appInfo?.cliVersion ? formatVersion(appInfo.cliVersion) : 'Unknown'}
            available={cliAvailable}
            releasesUrl={CLI_REPO}
            rowsStale={rowsStale}
            checkFailed={cliCheckFailed}
            action={
              canStartUpdate && cliAvailable ? (
                <Button
                  size="sm"
                  variant={bothAvailable ? 'secondary' : 'primary'}
                  onClick={() => void runCli()}
                >
                  Update CLI
                </Button>
              ) : null
            }
          />
        </SettingsList>
      </SettingsSection>

      <SettingsSection
        title="Skills, agents and plugins"
        heading="h2"
        action={
          <CheckButton
            label="Check skills, agents and plugins"
            checking={resourcesCheckStartedAt !== null}
            onClick={() => void checkResources()}
          />
        }
      >
        <div aria-live="polite">
          <SettingsList className="relative isolate overflow-hidden">
            {resourcesCheckStartedAt !== null ? (
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
                    {`Checking every source... ${elapsed(resourcesCheckStartedAt, now)}`}
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
            {resourcesCheckCount > 0 && resourcesCheckStartedAt === null && (
              <span
                key={resourcesCheckCount}
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
          id="auto-update-check"
          label="Automatically check for updates"
          hint={
            policyError ??
            policy?.error ??
            (policy?.envOverride
              ? 'Controlled by SKILLSHARE_AUTO_UPDATE_CHECK.'
              : policy?.supported === false
                ? 'Upgrade the CLI to manage this shared preference.'
                : 'Shared with the CLI and web UI in config.yaml. Turning this off also hides version notifications and badges. Manual checks and upgrades remain available.')
          }
        >
          <Switch
            checked={policy?.enabled ?? false}
            disabled={!policy || !policy.supported || policy.envOverride || policySaving}
            labelledBy="auto-update-check"
            describedBy="auto-update-check-hint"
            onChange={async (enabled) => {
              setPolicySaving(true);
              setPolicyError(null);
              try {
                await setUpdatePolicy(enabled);
              } catch (err) {
                setPolicyError(err instanceof Error ? err.message : String(err));
              } finally {
                setPolicySaving(false);
              }
            }}
          />
        </SettingsRow>
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
