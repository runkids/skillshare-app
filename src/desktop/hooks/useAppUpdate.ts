import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';

export type UpdateCheckStatus = 'idle' | 'checking' | 'available' | 'up-to-date' | 'error';

interface UpdateState {
  status: UpdateCheckStatus;
  update: Update | null;
  error: string | null;
}

const INITIAL: UpdateState = { status: 'idle', update: null, error: null };

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
  };
}

export function resetAppUpdateForTests() {
  state = INITIAL;
  inflight = null;
  autoChecked = false;
}
