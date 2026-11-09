import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CLI_UPGRADE_STEP_EVENT, tauriBridge } from '../api/tauri-bridge';
import { checkVersionsNow } from './useUpdates';

export type CliUpgradeStatus = 'idle' | 'upgrading' | 'done' | 'up-to-date' | 'error';

interface CliUpgradeState {
  status: CliUpgradeStatus;
  /** The step the upgrade last reported, e.g. "Downloading v0.24.1...". */
  step: string | null;
  startedAt: number;
  newVersion: string | null;
  error: string | null;
  /** The app update runs after this one ("Update both"), so its steps show from the start. */
  withApp: boolean;
}

const INITIAL: CliUpgradeState = {
  status: 'idle',
  step: null,
  startedAt: 0,
  newVersion: null,
  error: null,
  withApp: false,
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
  /** The app update of "Update both". It starts in the same tick the CLI step finishes, so
   * no render shows the CLI done with the app not yet started. */
  then?: () => unknown;
}

/** Upgrade the CLI through the app: it stops the server, upgrades, and restarts it. Resolves to success. */
export async function upgradeCli({ oldVersion, refresh, reloadView, then }: UpgradeHooks) {
  if (state.status === 'upgrading') return false;
  const withApp = Boolean(then);
  setState({ status: 'upgrading', step: null, startedAt: Date.now(), error: null, withApp });
  try {
    const version = await tauriBridge.upgradeCli();
    await refresh();
    reloadView();
    void checkVersionsNow().catch(() => {});
    const changed = Boolean(version) && version !== oldVersion;
    setState({ status: changed ? 'done' : 'up-to-date', newVersion: changed ? version : null });
    then?.();
    return true;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (/already|up to date/i.test(msg)) {
      setState({ status: 'up-to-date' });
      then?.();
      return true;
    }
    setState({ status: 'error', error: msg });
    return false;
  }
}

/** Forget a finished upgrade, so a new check starts from the versions alone. */
export function clearCliUpgrade() {
  if (state.status !== 'upgrading') setState(INITIAL);
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
