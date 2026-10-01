import { useState, useCallback, useEffect, type ReactNode } from 'react';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import Card from '../../../components/Card';
import Button from '../../../components/Button';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useAppUpdate, type UpdateCheckStatus } from '../../hooks/useAppUpdate';
import { useUpdates } from '../../hooks/useUpdates';
import { useNavigate } from 'react-router-dom';

type UpdateStatus = UpdateCheckStatus | 'downloading' | 'installing' | 'complete';

const APP_REPO = 'https://github.com/runkids/skillshare-app';

/** The webview ignores target=_blank, so hand external links to the system browser. */
function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
      className="text-sm text-pencil-light hover:text-pencil underline"
    >
      {children}
    </a>
  );
}

export default function AboutSettings() {
  const { appInfo } = useTauri();
  const { cli: cliUpdate } = useUpdates();
  const navigate = useNavigate();
  const [logsError, setLogsError] = useState<string | null>(null);

  const openLogs = () => {
    setLogsError(null);
    tauriBridge.openLogsFolder().catch((err) => setLogsError(String(err)));
  };
  const [diagnosticsMessage, setDiagnosticsMessage] = useState<string | null>(null);
  const exportDiagnostics = () => {
    setDiagnosticsMessage(null);
    tauriBridge
      .exportDiagnostics()
      .then((path) => setDiagnosticsMessage(`Saved to ${path}`))
      .catch((err) => setDiagnosticsMessage(String(err)));
  };
  const { update: updateObj, status: checkStatus, error: checkError, recheck } = useAppUpdate();
  const [appVersion, setAppVersion] = useState('0.1.0');
  // Download/install phases are local; the check result comes from the shared hook.
  const [installStatus, setInstallStatus] = useState<UpdateStatus | null>(null);
  const [progress, setProgress] = useState(0);
  const [installError, setInstallError] = useState<string | null>(null);
  const status: UpdateStatus = installStatus ?? checkStatus;
  const error = installStatus ? installError : checkError;
  const newVersion = updateObj?.version ?? null;

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => {});
  }, []);

  const handleCheck = useCallback(async () => {
    setInstallStatus(null);
    setInstallError(null);
    await recheck();
  }, [recheck]);

  const handleRestart = useCallback(async () => {
    try {
      await relaunch();
    } catch {
      setInstallError('Failed to restart. Please close and reopen the app.');
    }
  }, []);

  const handleDownload = useCallback(async () => {
    if (!updateObj) return;
    setInstallStatus('downloading');
    setProgress(0);
    try {
      let downloaded = 0;
      let total = 0;
      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            setProgress(total > 0 ? Math.round((downloaded / total) * 100) : 0);
            break;
          case 'Finished':
            setProgress(100);
            setInstallStatus('installing');
            break;
        }
      });
      setInstallStatus('complete');
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
      setInstallStatus('error');
      return;
    }
    // The user asked for the update, so apply it now instead of waiting for another click.
    await handleRestart();
  }, [updateObj, handleRestart]);

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        About
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">App Version</p>
          <span className="text-sm text-pencil-light">v{appVersion}</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">CLI Version</p>
          <span className="flex items-center gap-2 text-sm text-pencil-light">
            {appInfo?.cliVersion || 'Unknown'}
            {cliUpdate && (
              <button
                type="button"
                onClick={() => navigate('/settings?tab=cli')}
                className="text-[11px] font-bold px-[7px] py-0.5 rounded-[var(--r-btn)] bg-[var(--accent-bg)] text-[var(--accent)] hover:underline"
              >
                {cliUpdate} available
              </button>
            )}
          </span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">GitHub</p>
          <ExternalLink href={APP_REPO}>runkids/skillshare-app</ExternalLink>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">CLI Repository</p>
          <ExternalLink href="https://github.com/runkids/skillshare">
            runkids/skillshare
          </ExternalLink>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">Release Notes</p>
          <ExternalLink href={`${APP_REPO}/releases/tag/v${appVersion}`}>
            v{appVersion}
          </ExternalLink>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Logs</p>
            <p className="text-xs text-pencil-light mt-0.5">
              {logsError ?? 'The app log and the CLI server output, for bug reports'}
            </p>
          </div>
          <Button size="sm" variant="secondary" onClick={openLogs}>
            Open logs folder
          </Button>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Diagnostics</p>
            <p className="text-xs text-pencil-light mt-0.5">
              {diagnosticsMessage ??
                'App, CLI and server details with recent logs, saved to Downloads'}
            </p>
          </div>
          <Button size="sm" variant="secondary" onClick={exportDiagnostics} className="shrink-0">
            Export diagnostics
          </Button>
        </div>
      </Card>

      {/* App Update */}
      <Card>
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-pencil">App Update</p>
              <p className="text-xs text-pencil-light mt-0.5">
                {status === 'checking' && 'Checking for updates...'}
                {status === 'up-to-date' && 'You are on the latest version'}
                {status === 'available' && `Version ${newVersion} is available`}
                {status === 'downloading' && `Downloading... ${progress}%`}
                {status === 'installing' && 'Installing update...'}
                {status === 'complete' && (error || 'Update installed. Restarting…')}
                {status === 'error' && (error || 'Update check failed')}
                {status === 'idle' && 'Check for new versions of skillshare App'}
              </p>
            </div>
            <div className="shrink-0">
              {status === 'idle' && (
                <Button size="sm" onClick={handleCheck}>
                  Check for Updates
                </Button>
              )}
              {status === 'checking' && (
                <Button size="sm" loading>
                  Checking
                </Button>
              )}
              {status === 'up-to-date' && (
                <Button size="sm" variant="secondary" disabled>
                  Up to date
                </Button>
              )}
              {status === 'available' && (
                <Button size="sm" onClick={handleDownload}>
                  Update Now
                </Button>
              )}
              {(status === 'downloading' || status === 'installing') && (
                <Button size="sm" loading>
                  {progress}%
                </Button>
              )}
              {status === 'complete' && (
                <Button size="sm" onClick={handleRestart}>
                  Restart
                </Button>
              )}
              {status === 'error' && (
                <Button size="sm" onClick={handleCheck}>
                  Retry
                </Button>
              )}
            </div>
          </div>

          {/* Rendered as text, so release notes cannot inject markup. */}
          {status === 'available' && updateObj?.body && (
            <div className="max-h-48 overflow-y-auto rounded-[var(--r-btn)] bg-muted p-3 text-xs text-pencil-light whitespace-pre-wrap">
              {updateObj.body}
            </div>
          )}

          {/* Progress bar */}
          {(status === 'downloading' || status === 'installing') && (
            <div className="w-full h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-pencil rounded-full transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
