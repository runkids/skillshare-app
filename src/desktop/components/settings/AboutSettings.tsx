import { useState, useEffect, type ReactNode } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink as ExternalIcon, FileText, Stethoscope } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import Button from '../../../components/Button';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useAppUpdate } from '../../hooks/useAppUpdate';
import { useUpdates } from '../../hooks/useUpdates';
import { SettingsList, SettingsSection, StatusDot } from './SettingsLayout';

const APP_REPO = 'https://github.com/runkids/skillshare-app';

/** The webview ignores target=_blank, so hand external links to the system browser. */
function ExternalLink({ href, label, hint }: { href: string; label: string; hint: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
      className="flex items-center gap-3.5 min-h-14 px-4 py-3 text-[var(--ink)] hover:bg-[var(--sel)]"
    >
      <span className="flex-1 min-w-0 flex flex-col gap-0.5">
        <span className="text-sm font-semibold">{label}</span>
        <span className="font-mono text-xs text-[var(--ink-3)]">{hint}</span>
      </span>
      <ExternalIcon size={15} className="text-[var(--ink-3)]" aria-hidden="true" />
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

export default function AboutSettings() {
  const { appInfo } = useTauri();
  const updates = useUpdates();
  const { available: appUpdate } = useAppUpdate();
  const navigate = useNavigate();
  const [appVersion, setAppVersion] = useState('0.1.0');
  const [logsError, setLogsError] = useState<string | null>(null);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState<string | null>(null);
  const hasUpdate = appUpdate || Boolean(updates.app || updates.cli);

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => {});
  }, []);

  const openLogs = () => {
    setLogsError(null);
    tauriBridge.openLogsFolder().catch((err) => setLogsError(String(err)));
  };
  const exportDiagnostics = () => {
    setDiagnosticsMessage(null);
    tauriBridge
      .exportDiagnostics()
      .then((path) => setDiagnosticsMessage(`Saved to ${path}`))
      .catch((err) => setDiagnosticsMessage(String(err)));
  };

  return (
    <div className="flex flex-col gap-7">
      <section
        aria-label="skillshare App"
        className="flex items-center gap-4 p-5 rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-box)]"
        style={{ border: 'var(--frame)' }}
      >
        <img src="/logo.png" alt="" className="w-12 h-12 rounded-xl" />
        <div className="flex-1 min-w-0 flex flex-col gap-0.5">
          <h1 className="m-0" style={{ font: 'var(--h2)', letterSpacing: 'var(--h2-track)' }}>
            skillshare App
          </h1>
          <span className="font-mono text-[13px] text-[var(--ink-2)]">
            v{appVersion} · CLI {appInfo?.cliVersion || 'unknown'}
          </span>
        </div>
        {hasUpdate ? (
          <StatusDot tone="accent">Update available</StatusDot>
        ) : (
          <StatusDot tone="ok">Up to date</StatusDot>
        )}
        <Button size="sm" variant="secondary" onClick={() => navigate('/settings?tab=updates')}>
          Updates
        </Button>
      </section>

      <SettingsSection title="Links">
        <SettingsList className="overflow-hidden">
          <ExternalLink href={APP_REPO} label="App repository" hint="runkids/skillshare-app" />
          <ExternalLink
            href="https://github.com/runkids/skillshare"
            label="CLI repository"
            hint="runkids/skillshare"
          />
          <ExternalLink
            href={`${APP_REPO}/releases/tag/v${appVersion}`}
            label="Release notes"
            hint={`v${appVersion}`}
          />
        </SettingsList>
      </SettingsSection>

      <SettingsSection title="Troubleshooting">
        <SettingsList>
          <div className="flex items-center gap-3.5 min-h-14 px-4 py-3">
            <Tile>
              <FileText size={16} />
            </Tile>
            <SettingsRowText
              label="Logs"
              hint={logsError ?? 'The app log and the CLI server output, for bug reports.'}
            />
            <Button size="sm" variant="secondary" onClick={openLogs}>
              Open logs folder
            </Button>
          </div>
          <div className="flex items-center gap-3.5 min-h-14 px-4 py-3">
            <Tile>
              <Stethoscope size={16} />
            </Tile>
            <SettingsRowText
              label="Diagnostics"
              hint={
                diagnosticsMessage ??
                'App, CLI and server details with recent logs, saved to Downloads.'
              }
            />
            <Button size="sm" variant="secondary" onClick={exportDiagnostics}>
              Export diagnostics
            </Button>
          </div>
        </SettingsList>
      </SettingsSection>
    </div>
  );
}

function SettingsRowText({ label, hint }: { label: string; hint: string }) {
  return (
    <span className="flex-1 min-w-0 flex flex-col gap-0.5">
      <span className="text-sm font-semibold text-[var(--ink)]">{label}</span>
      <span className="text-[13px] text-[var(--ink-2)]">{hint}</span>
    </span>
  );
}
