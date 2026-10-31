import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CLI_UPGRADE_STEP_EVENT, tauriBridge } from '../api/tauri-bridge';
import { checkUpdatesNow } from './useUpdates';

export type CliUpgradeStatus = 'idle' | 'upgrading' | 'done' | 'up-to-date' | 'error';

interface CliUpgradeState {
  status: CliUpgradeStatus;
  /** The step the upgrade last reported, e.g. "Downloading v0.24.1...". */
  step: string | null;
  startedAt: number;
  newVersion: string | null;
  error: string | null;
}

const INITIAL: CliUpgradeState = {
  status: 'idle',
  step: null,
  startedAt: 0,
  newVersion: null,
  error: null,
};

// Module-level store so the upgrade keeps its progress when the user leaves the Updates page.
let state = INITIAL;
let listening = false;
const listeners = new Set<() => void>();

function setState(next: Partial<CliUpgradeState>) {
  state = { ...state, ...next };
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

interface UpgradeHooks {
  oldVersion: string | null | undefined;
  /** Re-read app info after the upgrade, so the new version shows. */
  refresh: () => Promise<unknown>;
  /** Reload the web view, which now talks to the restarted server. */
  reloadView: () => void;
}

/** Upgrade the CLI through the app: it stops the server, upgrades, and restarts it. Resolves to success. */
export async function upgradeCli({ oldVersion, refresh, reloadView }: UpgradeHooks) {
  if (state.status === 'upgrading') return false;
  setState({ status: 'upgrading', step: null, startedAt: Date.now(), error: null });
  try {
    const version = await tauriBridge.upgradeCli();
    await refresh();
    reloadView();
    void checkUpdatesNow().catch(() => {});
    const changed = Boolean(version) && version !== oldVersion;
    setState({ status: changed ? 'done' : 'up-to-date', newVersion: changed ? version : null });
    return true;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (/already|up to date/i.test(msg)) {
      setState({ status: 'up-to-date' });
      return true;
    }
    setState({ status: 'error', error: msg });
    return false;
  }
}

export function useCliUpgrade() {
  useEffect(() => {
    if (listening || !isTauri()) return;
    listening = true;
    void listen<string>(CLI_UPGRADE_STEP_EVENT, (e) => setState({ step: e.payload }));
  }, []);
  return useSyncExternalStore(subscribe, () => state);
}

export function resetCliUpgradeForTests() {
  state = INITIAL;
  listening = false;
}
