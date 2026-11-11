import { useEffect, useMemo, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, type AvailableUpdates } from '../api/tauri-bridge';
import { currentUpdatePolicy, useUpdatePolicy } from './useUpdatePolicy';

const NONE: AvailableUpdates = {
  cli: null,
  app: null,
  skills: [],
  repositories: [],
  agents: [],
  plugins: [],
};

// Module-level store fed by the Rust background check (launch, then every 24h).
let state = NONE;
let results = NONE;
let manualVersions: Pick<AvailableUpdates, 'cli' | 'app'> | null = null;
let started = false;
const listeners = new Set<() => void>();

function publish(next: AvailableUpdates, manual = false) {
  if (manual) manualVersions = { cli: next.cli, app: next.app };
  else if (currentUpdatePolicy()?.enabled) manualVersions = null;
  state = next;
  results = manualVersions ? { ...next, ...manualVersions } : next;
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function start() {
  if (started || !isTauri()) return;
  started = true;
  void listen<AvailableUpdates>('updates-available', (e) => publish(e.payload));
  tauriBridge
    .getAvailableUpdates()
    .then(publish)
    .catch(() => {});
}

let versionCheck: Promise<void> | null = null;
let versionCheckAutomatic = false;

/** Re-check the app and CLI versions now. Callers that overlap share one check. */
export function checkVersionsNow(automatic = false): Promise<void> {
  if (versionCheck && versionCheckAutomatic && !automatic) {
    return versionCheck.then(
      () => checkVersionsNow(false),
      () => checkVersionsNow(false)
    );
  }
  if (versionCheck) return versionCheck;
  versionCheckAutomatic = automatic;
  versionCheck = tauriBridge
    .checkVersionsNow(automatic)
    .then((next) => publish(next, !automatic))
    .finally(() => {
      versionCheck = null;
    });
  return versionCheck;
}

/** Re-check skills, agents and plugins now. Slow: the CLI fetches every source. */
export function checkResourcesNow() {
  return tauriBridge.checkResourcesNow().then((next) => publish(next));
}

/** Newer CLI/app versions (`null` = none known) and per-kind resources with updates found by the last check. */
export function useVersionResults() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => results);
}

export function useUpdates() {
  useEffect(start, []);
  const updates = useSyncExternalStore(subscribe, () => state);
  const policy = useUpdatePolicy();
  return useMemo(
    () => (policy?.enabled ? updates : { ...updates, cli: null, app: null }),
    [updates, policy?.enabled]
  );
}

export function resetUpdatesForTests() {
  state = NONE;
  results = NONE;
  manualVersions = null;
  started = false;
  versionCheck = null;
  versionCheckAutomatic = false;
}
