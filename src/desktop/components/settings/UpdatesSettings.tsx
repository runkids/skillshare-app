import { useEffect, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { AppWindow, Check, Download, Terminal } from 'lucide-react';
import Button from '../../../components/Button';
import Switch from '../../../components/Switch';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';
import { useAppUpdate } from '../../hooks/useAppUpdate';
import { upgradeCli, useCliUpgrade } from '../../hooks/useCliUpgrade';
import { checkUpdatesNow, useUpdates } from '../../hooks/useUpdates';
import { releaseNoteItems } from '../../utils/release-notes';
import {
  SettingsHeader,
  SettingsList,
  SettingsRow,
  SettingsSection,
  StatusDot,
} from './SettingsLayout';

const APP_REPO = 'https://github.com/runkids/skillshare-app';
const CLI_REPO = 'https://github.com/runkids/skillshare';
const v = (version: string) => `v${version.replace(/^v/, '')}`;
const mb = (bytes: number) => (bytes / 1048576).toFixed(1);

/** The webview ignores target=_blank, so hand external links to the system browser. */
function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
      className="text-[13px] text-[var(--accent)] hover:underline"
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

type StepState = 'done' | 'running' | 'pending' | 'failed';

function Step({
  state,
  title,
  children,
}: {
  state: StepState;
  title: ReactNode;
  children?: ReactNode;
}) {
  const marker = {
    done: (
      <span className="w-6 h-6 rounded-full bg-[var(--ok-bg)] text-[var(--ok)] flex items-center justify-center">
        <Check size={14} strokeWidth={2.5} />
      </span>
    ),
    running: (
      <span className="w-6 h-6 rounded-full border-2 border-[var(--ink)] flex items-center justify-center">
        <i className="w-2 h-2 rounded-full bg-[var(--ink)]" />
      </span>
    ),
    pending: <span className="w-6 h-6 rounded-full border-2 border-[var(--line-2)]" />,
    failed: (
      <span className="w-6 h-6 rounded-full bg-[var(--bad-bg)] text-[var(--bad)] flex items-center justify-center font-bold text-sm">
        !
      </span>
    ),
  }[state];
  return (
    <li className="flex items-start gap-3.5 py-3.5">
      <span className="shrink-0" aria-hidden="true">
        {marker}
      </span>
      <div className="flex-1 min-w-0 flex flex-col gap-1.5">
        <span
          className={`text-sm font-semibold ${state === 'pending' ? 'text-[var(--ink-2)]' : 'text-[var(--ink)]'}`}
        >
          {title}
        </span>
        {children}
      </div>
    </li>
  );
}

function elapsedLabel(startedAt: number, now: number) {
  const s = Math.max(0, Math.floor((now - startedAt) / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export default function UpdatesSettings() {
  const navigate = useNavigate();
  const { appInfo, refresh } = useTauri();
  const { reloadView } = useProjects();
  const updates = useUpdates();
  const app = useAppUpdate();
  const cli = useCliUpgrade();
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
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

  const cliRunning = cli.status === 'upgrading';
  useEffect(() => {
    if (!cliRunning) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [cliRunning]);

  const appNew = app.version;
  const cliNew = cli.status === 'done' || cli.status === 'up-to-date' ? null : updates.cli;
  const appBusy = app.phase !== 'idle' && app.phase !== 'error';
  const busy = appBusy || cliRunning;
  const both = Boolean(appNew && cliNew) && !busy;
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

  const runCli = () =>
    upgradeCli({ oldVersion: appInfo?.cliVersion, refresh, reloadView: () => reloadView() });
  const updateBoth = async () => {
    if (await runCli()) await app.install();
  };
  const checkNow = async () => {
    setChecking(true);
    await Promise.allSettled([app.recheck(), checkUpdatesNow()]);
    setChecking(false);
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

  const anyNew = Boolean(appNew || cliNew || resources.length);
  const subtitle = busy
    ? 'Updating. You can leave this page; it keeps going.'
    : checking || app.status === 'checking'
      ? 'Checking for new versions…'
      : anyNew
        ? 'The app checks when it starts and once a day.'
        : 'Everything is up to date. The app checks when it starts and once a day.';

  const showProgress = cli.status !== 'idle' || app.phase !== 'idle';
  const appNotes = releaseNoteItems(app.update?.body).slice(0, 5);
  const appPct = app.total > 0 ? Math.min(100, Math.round((app.downloaded / app.total) * 100)) : 0;

  const appSteps = (): ReactNode => {
    if (app.phase === 'idle') return null;
    const downloadState: StepState =
      app.phase === 'downloading' ? 'running' : app.phase === 'error' ? 'failed' : 'done';
    return (
      <>
        <Step
          state={downloadState}
          title={
            app.phase === 'error'
              ? `skillshare App ${appNew ? v(appNew) : ''} did not install`
              : `Download skillshare App ${appNew ? v(appNew) : ''}`
          }
        >
          {app.phase === 'downloading' && (
            <>
              <div
                role="progressbar"
                aria-label="Download"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={appPct}
                className="h-2 rounded-full bg-[var(--sunken)] overflow-hidden"
              >
                <div
                  className="h-full rounded-full bg-[var(--ink)] transition-[width] duration-300"
                  style={{ width: `${appPct}%` }}
                />
              </div>
              <span className="text-[13px] text-[var(--ink-2)]">
                {app.total > 0
                  ? `${mb(app.downloaded)} of ${mb(app.total)} MB`
                  : `${mb(app.downloaded)} MB`}
              </span>
            </>
          )}
          {app.phase === 'error' && (
            <span className="text-[13px] text-[var(--bad)]">{app.installError}</span>
          )}
        </Step>
        {app.phase !== 'error' && (
          <Step
            state={
              app.phase === 'installing'
                ? 'running'
                : app.phase === 'downloading'
                  ? 'pending'
                  : 'done'
            }
            title="Install"
          />
        )}
        {app.phase !== 'error' && (
          <Step
            state={
              app.phase === 'restart-failed'
                ? 'failed'
                : app.phase === 'restarting'
                  ? 'running'
                  : 'pending'
            }
            title={app.phase === 'restart-failed' ? 'Restart to finish' : 'Restart'}
          >
            <span className="text-[13px] text-[var(--ink-2)]">
              {app.phase === 'restart-failed'
                ? 'The update is installed but the app could not restart itself.'
                : 'The app restarts by itself and reopens. You can keep working until then.'}
            </span>
          </Step>
        )}
      </>
    );
  };

  return (
    <div className="flex flex-col gap-7">
      <SettingsHeader
        title="Updates"
        subtitle={subtitle}
        action={
          <Button
            size="sm"
            variant="secondary"
            onClick={checkNow}
            loading={checking}
            disabled={busy}
          >
            Check now
          </Button>
        }
      />

      {both && (
        <section
          aria-label="Update the app and the CLI"
          className="flex items-center gap-4 px-5 py-[18px] rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)]"
          style={{ border: 'var(--frame)' }}
        >
          <span className="w-10 h-10 shrink-0 flex items-center justify-center rounded-[var(--r-ctl)] bg-[var(--accent-bg)] text-[var(--accent)]">
            <Download size={20} />
          </span>
          <div className="flex-1 min-w-0 flex flex-col gap-0.5">
            <span style={{ font: 'var(--h2)', letterSpacing: 'var(--h2-track)' }}>
              New versions of the app and the CLI
            </span>
            <span className="text-[13px] text-[var(--ink-2)]">
              The CLI updates first. Then the app installs its update and restarts.
            </span>
          </div>
          <Button size="lg" onClick={updateBoth}>
            Update both
          </Button>
        </section>
      )}

      {showProgress && (
        <section aria-label="Update progress" aria-live="polite">
          <ol
            className="m-0 list-none px-5 py-1.5 rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)] [&>*+*]:[border-top:var(--sep)]"
            style={{ border: 'var(--frame)' }}
          >
            {cli.status !== 'idle' && (
              <Step
                state={
                  cli.status === 'upgrading'
                    ? 'running'
                    : cli.status === 'error'
                      ? 'failed'
                      : 'done'
                }
                title={
                  cli.status === 'upgrading'
                    ? 'Updating the CLI'
                    : cli.status === 'done'
                      ? `CLI updated to ${cli.newVersion ? v(cli.newVersion) : 'the latest version'}`
                      : cli.status === 'up-to-date'
                        ? 'The CLI is already on the latest version'
                        : 'The CLI update failed'
                }
              >
                <span
                  className={`text-[13px] ${cli.status === 'error' ? 'text-[var(--bad)]' : 'text-[var(--ink-2)]'}`}
                >
                  {cli.status === 'upgrading' &&
                    `${cli.step ?? 'Starting…'} ${elapsedLabel(cli.startedAt, now)}`}
                  {cli.status === 'done' && 'The server restarted on the new version.'}
                  {cli.status === 'error' && cli.error}
                </span>
              </Step>
            )}
            {appSteps()}
          </ol>
        </section>
      )}

      <SettingsSection title="Versions" heading="h2">
        <SettingsList>
          <div>
            <div className="flex items-center gap-3.5 px-4 py-4">
              <Tile>
                <AppWindow size={16} />
              </Tile>
              <div className="flex-1 min-w-0 flex flex-col gap-0.5">
                <span className="text-sm font-semibold">skillshare App</span>
                <span className="font-mono text-[13px] text-[var(--ink-2)]">
                  {appVersion ? v(appVersion) : '…'}
                  {appNew && (
                    <>
                      {' '}
                      <span className="text-[var(--ink-3)]">→</span>{' '}
                      <b className="text-[var(--ink)]">{v(appNew)}</b>
                    </>
                  )}
                </span>
              </div>
              {appNew && app.phase === 'idle' && (
                <Button
                  size="sm"
                  variant={both || cliNew ? 'secondary' : 'primary'}
                  onClick={app.install}
                >
                  Update and restart
                </Button>
              )}
              {app.phase === 'error' && (
                <Button size="sm" onClick={app.install}>
                  Retry
                </Button>
              )}
              {app.phase === 'restart-failed' && (
                <Button size="sm" onClick={app.restart}>
                  Restart now
                </Button>
              )}
              {!appNew && app.status !== 'checking' && <StatusDot tone="ok">Up to date</StatusDot>}
            </div>
            {appNew && (
              <div
                className="flex flex-col gap-1.5 px-4 pt-3.5 pb-4"
                style={{ borderTop: 'var(--sep)' }}
              >
                <span className="text-[13px] font-semibold">What’s new</span>
                {appNotes.length > 0 && (
                  <ul className="m-0 pl-[18px] text-[13px] text-[var(--ink-2)] flex flex-col gap-[3px]">
                    {appNotes.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                )}
                <span>
                  <ExternalLink href={`${APP_REPO}/releases/tag/${v(appNew)}`}>
                    Full release notes
                  </ExternalLink>
                </span>
              </div>
            )}
          </div>

          <div>
            <div className="flex items-center gap-3.5 px-4 py-4">
              <Tile>
                <Terminal size={16} />
              </Tile>
              <div className="flex-1 min-w-0 flex flex-col gap-0.5">
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
                  {cliNew && (
                    <>
                      {' '}
                      <span className="text-[var(--ink-3)]">→</span>{' '}
                      <b className="text-[var(--ink)]">{v(cliNew)}</b>
                    </>
                  )}
                </span>
              </div>
              {cliNew && !cliRunning && cli.status !== 'error' && (
                <Button
                  size="sm"
                  variant={both || appNew ? 'secondary' : 'primary'}
                  onClick={runCli}
                >
                  Update CLI
                </Button>
              )}
              {cli.status === 'error' && (
                <Button size="sm" onClick={runCli}>
                  Retry
                </Button>
              )}
              {cliRunning && <StatusDot tone="accent">Updating</StatusDot>}
              {!cliNew && !cliRunning && cli.status !== 'error' && (
                <StatusDot tone="ok">Up to date</StatusDot>
              )}
            </div>
            {cliNew && (
              <div
                className="flex items-center gap-3 px-4 py-3"
                style={{ borderTop: 'var(--sep)' }}
              >
                <span className="flex-1 text-[13px] text-[var(--ink-2)]">
                  The dashboard reloads when the CLI update finishes.
                </span>
                <ExternalLink href={`${CLI_REPO}/releases/tag/${v(cliNew)}`}>
                  Release notes
                </ExternalLink>
              </div>
            )}
          </div>
        </SettingsList>
      </SettingsSection>

      <SettingsSection title="Skills, agents and plugins" heading="h2">
        <SettingsList>
          {resources.length === 0 ? (
            <SettingsRow
              label="No updates"
              hint="Installed skills, agents and plugins match their sources."
            />
          ) : (
            resources.map((r) => (
              <SettingsRow
                key={r.one}
                label={`${r.names.length} ${r.names.length === 1 ? r.one : r.many}`}
                hint={<span className="font-mono">{r.names.join(', ')}</span>}
              >
                <Button size="sm" variant="secondary" onClick={() => navigate(r.path)}>
                  Review
                </Button>
              </SettingsRow>
            ))
          )}
        </SettingsList>
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
