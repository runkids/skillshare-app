import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useTauri } from '../context/TauriContext';
import { useProjects } from '../context/ProjectContext';

// A quiet dot while the server is up; text only when it needs attention.
export default function ServerStatus() {
  const { appInfo } = useTauri();
  const { reloadView } = useProjects();
  // `appInfo` is read once, so follow the supervisor's events to notice a dead server.
  const [stopped, setStopped] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    const offStopped = listen('server-stopped', () => setStopped(true));
    const offRestarted = listen('server-restarted', () => setStopped(false));
    return () => {
      void offStopped.then((off) => off());
      void offRestarted.then((off) => off());
    };
  }, []);

  if (stopped) {
    return (
      <button
        type="button"
        // The header reload restarts a server that is not healthy.
        onClick={() => {
          setStopped(false);
          reloadView();
        }}
        className="flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] bg-[var(--bad-bg)] text-[var(--bad)] text-xs hover:opacity-80 transition-opacity"
        title="The server kept exiting. Click to restart it."
      >
        <span className="w-1.5 h-1.5 rounded-full bg-[var(--bad)]" />
        Server stopped
      </button>
    );
  }

  if (!appInfo?.serverRunning) return null;
  const label = `Server running on port ${appInfo.serverPort}`;
  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      className="w-[30px] h-[30px] flex items-center justify-center"
    >
      <span className="w-2 h-2 rounded-full bg-[var(--ok)]" />
    </span>
  );
}
