import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateCheckStatus = 'idle' | 'checking' | 'available' | 'up-to-date' | 'error';
/** Where installing the available update is: download, install, then the automatic restart. */
export type InstallPhase =
  | 'idle'
  | 'downloading'
  | 'installing'
  | 'restarting'
  | 'restart-failed'
  | 'error';

interface UpdateState {
  status: UpdateCheckStatus;
  update: Update | null;
  error: string | null;
  phase: InstallPhase;
  downloaded: number;
  /** Download size in bytes; 0 when the server did not send one. */
  total: number;
  installError: string | null;
}

const INITIAL: UpdateState = {
  status: 'idle',
  update: null,
  error: null,
  phase: 'idle',
  downloaded: 0,
  total: 0,
  installError: null,
};

/** localStorage key for the update being applied, read after the restart to say what changed. */
export const JUST_UPDATED_KEY = 'skillshare-just-updated';
export interface JustUpdated {
  version: string;
  notes: string;
}

// Module-level store: the update check runs at most once per app session and the
// resulting Update object is shared by the settings nav badge and AboutSettings.
let state = INITIAL;
let inflight: Promise<void> | null = null;
let autoChecked = false;
const listeners = new Set<() => void>();

function setState(next: Partial<UpdateState>) {
  state = { ...state, ...next };
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function runCheck(): Promise<void> {
  if (inflight) return inflight;
  setState({ status: 'checking', error: null });
  inflight = (async () => {
    try {
      const update = await check();
      if (update) {
        setState({ status: 'available', update });
      } else {
        setState({ status: 'up-to-date', update: null });
        setTimeout(() => {
          if (state.status === 'up-to-date') setState({ status: 'idle' });
        }, 3000);
      }
    } catch (err) {
      setState({
        status: 'error',
        update: null,
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

/** Download and install the available update, then restart into it. */
async function install() {
  const update = state.update;
  if (!update || (state.phase !== 'idle' && state.phase !== 'error')) return;
  setState({ phase: 'downloading', downloaded: 0, total: 0, installError: null });
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') setState({ total: event.data.contentLength ?? 0 });
      if (event.event === 'Progress')
        setState({ downloaded: state.downloaded + event.data.chunkLength });
      if (event.event === 'Finished') setState({ phase: 'installing' });
    });
  } catch (err) {
    setState({ phase: 'error', installError: err instanceof Error ? err.message : String(err) });
    return;
  }
  try {
    const note: JustUpdated = { version: update.version, notes: update.body ?? '' };
    localStorage.setItem(JUST_UPDATED_KEY, JSON.stringify(note));
  } catch {
    // Storage can be unavailable; the update still applies, only the notice is lost.
  }
  setState({ phase: 'restarting' });
  try {
    await relaunch();
  } catch {
    setState({ phase: 'restart-failed' });
  }
}

export function useAppUpdate() {
  const s = useSyncExternalStore(subscribe, () => state);

  useEffect(() => {
    // The browser preview has no updater backend; leave it idle until asked.
    if (autoChecked || !isTauri()) return;
    autoChecked = true;
    void runCheck();
  }, []);

  return {
    available: s.status === 'available',
    version: s.update?.version ?? null,
    update: s.update,
    status: s.status,
    error: s.error,
    recheck: runCheck,
    phase: s.phase,
    downloaded: s.downloaded,
    total: s.total,
    installError: s.installError,
    install,
    restart: () => relaunch().catch(() => setState({ phase: 'restart-failed' })),
  };
}

export function resetAppUpdateForTests() {
  state = INITIAL;
  inflight = null;
  autoChecked = false;
}
