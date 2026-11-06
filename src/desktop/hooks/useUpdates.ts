import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, type AvailableUpdates } from '../api/tauri-bridge';

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
let started = false;
const listeners = new Set<() => void>();

function publish(next: AvailableUpdates) {
  state = next;
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

/** Re-check the app and CLI versions now. Callers that overlap share one check. */
export function checkVersionsNow() {
  versionCheck ??= tauriBridge
    .checkVersionsNow()
    .then(publish)
    .finally(() => {
      versionCheck = null;
    });
  return versionCheck;
}

/** Re-check skills, agents and plugins now. Slow: the CLI fetches every source. */
export function checkResourcesNow() {
  return tauriBridge.checkResourcesNow().then(publish);
}

/** Newer CLI/app versions (`null` = none known) and per-kind resources with updates found by the last check. */
export function useUpdates() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => state);
}
