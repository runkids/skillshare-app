import { useEffect, useState } from 'react';
import Card from '../../../components/Card';
import Badge from '../../../components/Badge';
import Button from '../../../components/Button';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';
import { tauriBridge } from '../../api/tauri-bridge';
import TerminalAccess from '../TerminalAccess';
import { checkUpdatesNow, useUpdates } from '../../hooks/useUpdates';

type UpgradeStatus = 'idle' | 'upgrading' | 'success' | 'up-to-date' | 'error';

export default function CliSettings() {
  const { appInfo, refresh } = useTauri();
  const { reloadView } = useProjects();
  const { cli: cliUpdate } = useUpdates();
  const [status, setStatus] = useState<UpgradeStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [newVersion, setNewVersion] = useState<string | null>(null);
  const [cliPath, setCliPath] = useState<string | null>(null);
  // Seconds since Upgrade was clicked; the download alone can take minutes.
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (status !== 'upgrading') return;
    const timer = window.setInterval(() => setElapsed((s) => s + 1), 1000);
    return () => window.clearInterval(timer);
  }, [status]);

  useEffect(() => {
    tauriBridge
      .detectCli()
      .then(setCliPath)
      .catch(() => {});
  }, []);

  const handleUpgrade = async () => {
    setElapsed(0);
    setStatus('upgrading');
    setError(null);
    try {
      const oldVersion = appInfo?.cliVersion;
      // Stops the server, upgrades, then restarts it on the new binary for the active project
      const updatedVersion = await tauriBridge.upgradeCli();
      await refresh();
      reloadView();
      void checkUpdatesNow().catch(() => {});

      if (updatedVersion && updatedVersion !== oldVersion) {
        setNewVersion(updatedVersion);
        setStatus('success');
      } else {
        setStatus('up-to-date');
      }
      setTimeout(() => setStatus('idle'), 5000);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      // Check if "already up to date" or similar
      if (msg.toLowerCase().includes('already') || msg.toLowerCase().includes('up to date')) {
        setStatus('up-to-date');
        setTimeout(() => setStatus('idle'), 3000);
      } else {
        setError(msg);
        setStatus('error');
      }
    }
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        CLI
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Version</p>
            <p className="text-xs text-pencil-light mt-0.5">Currently installed CLI version</p>
          </div>
          <span className="text-sm text-pencil">{appInfo?.cliVersion || 'Unknown'}</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Source</p>
            <p className="text-xs text-pencil-light mt-0.5">How the CLI was installed</p>
          </div>
          <Badge size="sm">{appInfo?.cliSource || 'unknown'}</Badge>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Update</p>
            <p className="text-xs text-pencil-light mt-0.5">
              {status === 'idle' &&
                (cliUpdate ? `${cliUpdate} is available` : 'Upgrade CLI to the latest version')}
              {status === 'upgrading' &&
                `Downloading and installing… ${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')}. This can take a few minutes.`}
              {status === 'success' && `Updated to ${newVersion}`}
              {status === 'up-to-date' && 'Already on the latest version'}
              {status === 'error' && (error || 'Upgrade failed')}
            </p>
          </div>
          <div className="shrink-0">
            {(status === 'idle' || status === 'error') && (
              <Button size="sm" onClick={handleUpgrade}>
                {status === 'error' ? 'Retry' : 'Upgrade CLI'}
              </Button>
            )}
            {status === 'upgrading' && (
              <Button size="sm" loading>
                Upgrading
              </Button>
            )}
            {status === 'success' && (
              <Button size="sm" variant="secondary" disabled>
                Updated
              </Button>
            )}
            {status === 'up-to-date' && (
              <Button size="sm" variant="secondary" disabled>
                Up to date
              </Button>
            )}
          </div>
        </div>
      </Card>

      {cliPath && (
        <Card>
          <p className="text-sm font-medium text-pencil">Terminal</p>
          <p className="text-xs text-pencil-light mt-0.5 mb-3">Run skillshare outside the app</p>
          <TerminalAccess
            cliPath={cliPath}
            fallback={
              <p className="text-xs text-pencil-light">
                skillshare is on your PATH and ready to use.
              </p>
            }
          />
        </Card>
      )}
    </div>
  );
}
