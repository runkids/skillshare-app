import { useState, useEffect, useRef, useCallback } from 'react';
import Spinner from '../../components/Spinner';
import Button from '../../components/Button';
import { useProjects } from '../context/ProjectContext';
import { tauriBridge } from '../api/tauri-bridge';
import { useTauri } from '../context/TauriContext';

const HEALTH_POLL_INTERVAL = 30_000;
const HEALTH_FAIL_THRESHOLD = 3;

type Status = 'loading' | 'ready' | 'error' | 'server-down';

export default function CliWebView() {
  const { appInfo, refresh: refreshAppInfo } = useTauri();
  const { switching, activeProject } = useProjects();
  const [status, setStatus] = useState<Status>('loading');
  const [iframeUrl, setIframeUrl] = useState<string | null>(null);
  const [iframeKey, setIframeKey] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [restarting, setRestarting] = useState(false);
  const failCount = useRef(0);
  const pollRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);
  const startAttempted = useRef(false);

  // Try to start server if no port available on mount
  useEffect(() => {
    if (appInfo?.serverPort) {
      setIframeUrl(`http://localhost:${appInfo.serverPort}`);
      setStatus('ready');
      failCount.current = 0;
      startAttempted.current = false;
      return;
    }

    // No port — try starting the server (only once)
    if (startAttempted.current || switching) return;
    startAttempted.current = true;

    (async () => {
      try {
        const cliPath = await tauriBridge.detectCli();
        if (!cliPath) {
          setError('CLI not found. Please reinstall.');
          setStatus('error');
          return;
        }
        const projectDir = activeProject?.path;
        const port = await tauriBridge.startServer(cliPath, projectDir);
        setIframeUrl(`http://localhost:${port}`);
        setStatus('ready');
        await refreshAppInfo();
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setStatus('error');
      }
    })();
  }, [appInfo?.serverPort, switching, activeProject, refreshAppInfo]);

  // Health check polling when server is ready
  useEffect(() => {
    if (status !== 'ready' || !iframeUrl) return;
    pollRef.current = setInterval(async () => {
      try {
        const healthy = await tauriBridge.healthCheck();
        if (healthy) {
          failCount.current = 0;
        } else {
          failCount.current++;
          if (failCount.current >= HEALTH_FAIL_THRESHOLD) setStatus('server-down');
        }
      } catch {
        failCount.current++;
        if (failCount.current >= HEALTH_FAIL_THRESHOLD) setStatus('server-down');
      }
    }, HEALTH_POLL_INTERVAL);
    return () => clearInterval(pollRef.current);
  }, [status, iframeUrl]);

  // Force iframe reload when switching completes
  const prevSwitching = useRef(false);
  useEffect(() => {
    if (prevSwitching.current && !switching) {
      startAttempted.current = false; // allow re-attempt if port changed
      refreshAppInfo();
      setIframeKey((k) => k + 1);
    }
    prevSwitching.current = switching;
  }, [switching, refreshAppInfo]);

  const handleRestart = useCallback(async () => {
    setRestarting(true);
    setError(null);
    try {
      // Stop any existing server first
      await tauriBridge.stopServer().catch(() => {});
      const cliPath = await tauriBridge.detectCli();
      if (!cliPath) throw new Error('CLI not found');
      const projectDir = activeProject?.path;
      const port = await tauriBridge.startServer(cliPath, projectDir);
      setIframeUrl(`http://localhost:${port}`);
      setStatus('ready');
      failCount.current = 0;
      await refreshAppInfo();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus('error');
    } finally {
      setRestarting(false);
    }
  }, [activeProject, refreshAppInfo]);

  // Switching state
  if (switching) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 bg-paper">
        <Spinner size="lg" />
        <span className="text-pencil-light text-sm">
          Switching to {activeProject?.name || 'project'}...
        </span>
      </div>
    );
  }

  // Loading state
  if (status === 'loading') {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 bg-paper">
        <Spinner size="lg" />
        <span className="text-pencil-light text-sm">Starting server...</span>
      </div>
    );
  }

  // Error or server-down state
  if (status === 'error' || status === 'server-down') {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-4 bg-paper">
        <p className="text-pencil font-medium">
          {status === 'error' ? 'Server failed to start' : 'Server disconnected'}
        </p>
        {error && (
          <p className="text-pencil-light text-sm max-w-md text-center">{error}</p>
        )}
        <Button onClick={handleRestart} loading={restarting}>
          {status === 'error' ? 'Retry' : 'Restart Server'}
        </Button>
      </div>
    );
  }

  return (
    <iframe
      key={`${iframeUrl}-${iframeKey}`}
      src={iframeUrl!}
      className="flex-1 w-full border-0"
      allow="clipboard-write"
      title="Skillshare UI"
    />
  );
}
