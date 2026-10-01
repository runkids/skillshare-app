import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AlertTriangle, Check, RotateCw, Unplug } from 'lucide-react';
import Button from '../../components/Button';
import { useProjects } from '../context/ProjectContext';
import { tauriBridge } from '../api/tauri-bridge';
import { useTauri } from '../context/TauriContext';
import { useTheme } from '../../context/useTheme';

const HEALTH_POLL_INTERVAL = 30_000;
const HEALTH_FAIL_THRESHOLD = 3;

type Status = 'loading' | 'ready' | 'error' | 'server-down';

const CARD =
  'w-full max-w-[520px] bg-[var(--surface)] border-[length:var(--bw)] border-[var(--line)] rounded-[var(--r-box)] shadow-[var(--sh-box)] p-9 flex flex-col motion-safe:animate-[fadeInUp_200ms_ease-out]';
const HEADING = 'text-[28px] font-bold tracking-[-0.022em] leading-[1.1] text-[var(--ink)]';

function StartingCard({
  title,
  cliVersion,
  scope,
}: {
  title: string;
  cliVersion?: string | null;
  scope?: string;
}) {
  return (
    <section aria-label="Starting" aria-busy="true" className={`${CARD} gap-[22px]`}>
      <div className="text-xs font-bold tracking-[.08em] text-[var(--ink-3)]">LAUNCHING</div>
      <h1 className={HEADING} style={{ fontFamily: 'var(--fh)' }}>
        {title}
      </h1>
      <ol className="flex flex-col gap-3.5 text-[15px]">
        <li className="flex items-center gap-3">
          <span className="w-7 h-7 rounded-full bg-[var(--ok)] text-[var(--surface)] flex items-center justify-center">
            <Check size={15} />
          </span>
          <span className="flex-1">Found CLI</span>
          {cliVersion && (
            <span className="font-mono text-xs text-[var(--ink-3)]">v{cliVersion}</span>
          )}
        </li>
        <li className="flex items-center gap-3 font-bold">
          <span className="w-7 h-7 rounded-full border-[3px] border-[var(--line)] border-t-[var(--accent)] animate-spin" />
          <span className="flex-1">Starting local server</span>
          {scope && <span className="text-xs font-normal text-[var(--ink-3)]">{scope}</span>}
        </li>
        <li className="flex items-center gap-3 text-[var(--ink-3)]">
          <span className="w-7 h-7 rounded-full border border-dashed border-[var(--line-2)]" />
          <span>Loading dashboard</span>
        </li>
      </ol>
      <div className="h-2 rounded-[var(--r-btn)] bg-[var(--sunken)] overflow-hidden">
        <div className="h-full w-[55%] rounded-[var(--r-btn)] bg-[var(--accent)]/40 motion-safe:animate-[shimmer_1.5s_ease-in-out_infinite]" />
      </div>
    </section>
  );
}

export default function CliWebView() {
  const navigate = useNavigate();
  const location = useLocation();
  const { appInfo, refresh: refreshAppInfo } = useTauri();
  const { switching, activeProject, reloadKey } = useProjects();
  const { style, setStyle, resolvedMode, setModePreference } = useTheme();
  const [status, setStatus] = useState<Status>('loading');
  const [iframeUrl, setIframeUrl] = useState<string | null>(null);
  const [iframeKey, setIframeKey] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [restarting, setRestarting] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const loadedIframeRef = useRef<HTMLIFrameElement | null>(null);
  const failCount = useRef(0);
  const pollRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);
  const startAttempted = useRef(false);

  // Derive CLI theme from shell's style + mode
  // CLI values: dark, light, playful, clean
  const cliTheme = useMemo(() => {
    if (style === 'playful') return 'playful';
    return resolvedMode === 'dark' ? 'dark' : 'clean';
  }, [style, resolvedMode]);

  // Listen for theme changes from CLI UI (iframe postMessage)
  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (event.data?.type !== 'theme-change') return;
      if (event.source !== iframeRef.current?.contentWindow) return;
      if (loadedIframeRef.current !== iframeRef.current) return;
      const theme = event.data.theme;
      // The CLI sends style and mode independently. Preserve the other axis.
      if (theme === 'playful' || theme === 'clean') {
        setStyle(theme);
      } else if ((theme === 'dark' || theme === 'light') && theme !== resolvedMode) {
        setModePreference(theme);
      }
    };
    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [setStyle, setModePreference, resolvedMode]);

  const pushTheme = useCallback(() => {
    iframeRef.current?.contentWindow?.postMessage(
      { type: 'theme-push', mode: resolvedMode, style },
      '*'
    );
  }, [resolvedMode, style]);

  // Push each axis independently, including Playful light/dark changes.
  useEffect(() => {
    if (loadedIframeRef.current === iframeRef.current) pushTheme();
  }, [pushTheme]);

  // Ref to capture current cliTheme for the initial URL without re-triggering server start
  const cliThemeRef = useRef(cliTheme);
  cliThemeRef.current = cliTheme;

  // A shell path other than "/" (e.g. /skills?tab=updates) opens that CLI UI page.
  const webPath = location.pathname === '/' ? '' : location.pathname;
  const webSearch = webPath ? location.search : '';
  // Remount on every navigation to a page, so it reopens even if the iframe moved on.
  const navKey = webPath ? location.key : '';

  // Read the theme each time the iframe (re)mounts. A stale ?theme= would make the
  // reloaded CLI UI restore the old style and post it back, undoing the user's choice.
  const iframeSrc = useMemo(() => {
    if (!iframeUrl) return null;
    const params = new URLSearchParams(webSearch);
    params.set('theme', cliThemeRef.current);
    return `${iframeUrl}${webPath}?${params}`;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- iframeKey and navKey mark a remount
  }, [iframeUrl, iframeKey, navKey, webPath, webSearch]);

  // Try to start server if no port available on mount
  useEffect(() => {
    if (!isTauri()) return;
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

  // The backend restarts a server that exited on its own, possibly on another port,
  // and gives up when it keeps exiting.
  useEffect(() => {
    if (!isTauri()) return;
    const restarted = listen<number>('server-restarted', (e) => {
      setIframeUrl(`http://localhost:${e.payload}`);
      setIframeKey((k) => k + 1);
      setError(null);
      setStatus('ready');
      failCount.current = 0;
      void refreshAppInfo();
    });
    const stopped = listen('server-stopped', () => {
      setError('The server kept exiting, so it was not restarted again. See server.log.');
      setStatus('server-down');
    });
    return () => {
      void restarted.then((off) => off());
      void stopped.then((off) => off());
    };
  }, [refreshAppInfo]);

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

  // Header reload: reload the page when the server is healthy, otherwise restart it.
  const prevReloadKey = useRef(reloadKey);
  useEffect(() => {
    if (prevReloadKey.current === reloadKey) return;
    prevReloadKey.current = reloadKey;
    (async () => {
      const healthy = await tauriBridge.healthCheck().catch(() => false);
      if (!healthy) {
        await handleRestart();
        return;
      }
      failCount.current = 0;
      setStatus('ready');
      await refreshAppInfo();
      setIframeKey((k) => k + 1);
    })();
  }, [reloadKey, handleRestart, refreshAppInfo]);

  if (!isTauri()) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 bg-paper">
        <p className="text-pencil font-medium">Browser preview</p>
        <p className="text-pencil-light text-sm">Desktop features require the Tauri app.</p>
      </div>
    );
  }

  // Switching state
  if (switching) {
    return (
      <div className="flex-1 flex items-center justify-center p-8 bg-[var(--bg)]">
        <StartingCard
          title={`Switching to ${activeProject?.name || 'project'}…`}
          cliVersion={appInfo?.cliVersion}
          scope={activeProject?.name}
        />
      </div>
    );
  }

  // Loading state
  if (status === 'loading') {
    return (
      <div className="flex-1 flex items-center justify-center p-8 bg-[var(--bg)]">
        <StartingCard
          title="Getting skillshare ready…"
          cliVersion={appInfo?.cliVersion}
          scope={activeProject?.name}
        />
      </div>
    );
  }

  // Error or server-down state
  if (status === 'error' || status === 'server-down') {
    const down = status === 'server-down';
    return (
      <div className="flex-1 flex items-center justify-center p-8 bg-[var(--bg)]">
        <section
          role="alert"
          aria-label={down ? 'Disconnected' : 'Failed to start'}
          className={`${CARD} gap-[18px]`}
        >
          <span className="w-[52px] h-[52px] rounded-[var(--r-box)] bg-[var(--bad-bg)] text-[var(--bad)] flex items-center justify-center">
            {down ? <Unplug size={24} /> : <AlertTriangle size={24} />}
          </span>
          <h1 className={HEADING} style={{ fontFamily: 'var(--fh)' }}>
            {down ? 'Lost touch with the server' : 'Server failed to start'}
          </h1>
          <p className="text-[15px] leading-[1.55] text-[var(--ink-2)]">
            {down
              ? 'The local skillshare server stopped answering health checks.'
              : 'The local skillshare server could not be started.'}{' '}
            Your skills and config are safe on disk.
          </p>
          <div className="flex items-center gap-3">
            <Button size="lg" onClick={handleRestart} loading={restarting}>
              {!restarting && <RotateCw size={16} />}
              {down ? 'Restart server' : 'Retry'}
            </Button>
            <Button size="lg" variant="secondary" onClick={() => navigate('/settings?tab=cli')}>
              CLI settings
            </Button>
          </div>
          {error && (
            <details className="rounded-[var(--r-box)] bg-[var(--sunken)] px-3.5 py-3">
              <summary className="text-[13px] font-semibold cursor-pointer text-[var(--ink)]">
                Technical details
              </summary>
              <pre className="mt-2.5 font-mono text-xs text-[var(--ink-2)] whitespace-pre-wrap">
                {error}
              </pre>
            </details>
          )}
        </section>
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      key={`${iframeUrl}-${iframeKey}-${navKey}`}
      src={iframeSrc!}
      className="flex-1 w-full border-0"
      onLoad={() => {
        loadedIframeRef.current = iframeRef.current;
        pushTheme();
      }}
      allow="clipboard-write"
      title="skillshare UI"
    />
  );
}
