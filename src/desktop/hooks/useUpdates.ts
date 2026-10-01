import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, type AvailableUpdates } from '../api/tauri-bridge';

const NONE: AvailableUpdates = { cli: null, app: null };

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

/** Re-check the CLI and app now, ignoring the 24h schedule. */
export function checkUpdatesNow() {
  return tauriBridge.checkUpdatesNow().then(publish);
}

/** Newer CLI/app versions found by the last check (`null` = none known). */
export function useUpdates() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => state);
}
