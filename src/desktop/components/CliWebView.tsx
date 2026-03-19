import { useState, useEffect, useRef, useCallback } from 'react';
import Spinner from '../../components/Spinner';
import Button from '../../components/Button';
import { useProjects } from '../hooks/useProjects';
import { tauriBridge } from '../api/tauri-bridge';
import { useTauri } from '../context/TauriContext';

const HEALTH_POLL_INTERVAL = 30_000;
const HEALTH_FAIL_THRESHOLD = 3;

export default function CliWebView() {
  const { appInfo, refresh: refreshAppInfo } = useTauri();
  const { switching, activeProject } = useProjects();
  const [iframeUrl, setIframeUrl] = useState<string | null>(null);
  const [serverDown, setServerDown] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const failCount = useRef(0);
  const pollRef = useRef<ReturnType<typeof setInterval>>();

  useEffect(() => {
    if (appInfo?.serverPort) {
      setIframeUrl(`http://localhost:${appInfo.serverPort}`);
      setServerDown(false);
      failCount.current = 0;
    }
  }, [appInfo?.serverPort]);

  useEffect(() => {
    if (!iframeUrl) return;
    pollRef.current = setInterval(async () => {
      try {
        const healthy = await tauriBridge.healthCheck();
        if (healthy) {
          failCount.current = 0;
          setServerDown(false);
        } else {
          failCount.current++;
          if (failCount.current >= HEALTH_FAIL_THRESHOLD) setServerDown(true);
        }
      } catch {
        failCount.current++;
        if (failCount.current >= HEALTH_FAIL_THRESHOLD) setServerDown(true);
      }
    }, HEALTH_POLL_INTERVAL);
    return () => clearInterval(pollRef.current);
  }, [iframeUrl]);

  useEffect(() => {
    if (!switching) {
      refreshAppInfo();
    }
  }, [switching, refreshAppInfo]);

  const handleRestart = useCallback(async () => {
    setRestarting(true);
    try {
      const cliPath = await tauriBridge.detectCli();
      if (!cliPath) throw new Error('CLI not found');
      const projectDir = activeProject?.path;
      const port = await tauriBridge.startServer(cliPath, projectDir);
      setIframeUrl(`http://localhost:${port}`);
      setServerDown(false);
      failCount.current = 0;
    } catch {
      // Stay in server-down state
    } finally {
      setRestarting(false);
    }
  }, [activeProject]);

  if (!iframeUrl || switching) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 bg-paper">
        <Spinner size="lg" />
        <span className="text-pencil-light text-sm">
          {switching
            ? `Switching to ${activeProject?.name || 'project'}...`
            : 'Starting server...'}
        </span>
      </div>
    );
  }

  if (serverDown) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-4 bg-paper">
        <p className="text-pencil font-medium">Server disconnected</p>
        <p className="text-pencil-light text-sm">
          The CLI server is no longer responding.
        </p>
        <Button onClick={handleRestart} loading={restarting}>
          Restart Server
        </Button>
      </div>
    );
  }

  return (
    <iframe
      key={iframeUrl}
      src={iframeUrl}
      className="flex-1 w-full border-0"
      allow="clipboard-write"
      title="Skillshare UI"
    />
  );
}
