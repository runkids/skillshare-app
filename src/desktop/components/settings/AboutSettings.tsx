import { useState, useCallback, useEffect } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';
import Card from '../../../components/Card';
import Button from '../../../components/Button';
import { useTauri } from '../../context/TauriContext';

type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'complete' | 'up-to-date' | 'error';

export default function AboutSettings() {
  const { appInfo } = useTauri();
  const [appVersion, setAppVersion] = useState('0.1.0');
  const [status, setStatus] = useState<UpdateStatus>('idle');
  const [newVersion, setNewVersion] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [updateObj, setUpdateObj] = useState<Update | null>(null);

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  const handleCheck = useCallback(async () => {
    setStatus('checking');
    setError(null);
    try {
      const update = await check();
      if (update) {
        setUpdateObj(update);
        setNewVersion(update.version);
        setStatus('available');
      } else {
        setStatus('up-to-date');
        setTimeout(() => setStatus('idle'), 3000);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, []);

  const handleDownload = useCallback(async () => {
    if (!updateObj) return;
    setStatus('downloading');
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
            setStatus('installing');
            break;
        }
      });
      setStatus('complete');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [updateObj]);

  const handleRestart = useCallback(async () => {
    try {
      await relaunch();
    } catch {
      setError('Failed to restart. Please close and reopen the app.');
    }
  }, []);

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
          <span className="text-sm text-pencil-light">{appInfo?.cliVersion || 'Unknown'}</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">GitHub</p>
          <a
            href="https://github.com/runkids/skillshare"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-pencil-light hover:text-pencil underline"
          >
            runkids/skillshare
          </a>
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
                {status === 'complete' && 'Update installed. Restart to apply.'}
                {status === 'error' && (error || 'Update check failed')}
                {status === 'idle' && 'Check for new versions of Skillshare App'}
              </p>
            </div>
            <div className="shrink-0">
              {status === 'idle' && (
                <Button size="sm" onClick={handleCheck}>Check for Updates</Button>
              )}
              {status === 'checking' && (
                <Button size="sm" loading>Checking</Button>
              )}
              {status === 'up-to-date' && (
                <Button size="sm" variant="secondary" disabled>Up to date</Button>
              )}
              {status === 'available' && (
                <Button size="sm" onClick={handleDownload}>Update Now</Button>
              )}
              {(status === 'downloading' || status === 'installing') && (
                <Button size="sm" loading>{progress}%</Button>
              )}
              {status === 'complete' && (
                <Button size="sm" onClick={handleRestart}>Restart</Button>
              )}
              {status === 'error' && (
                <Button size="sm" onClick={handleCheck}>Retry</Button>
              )}
            </div>
          </div>

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
