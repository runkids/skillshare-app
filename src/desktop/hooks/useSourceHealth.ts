import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, SOURCE_HEALTH_EVENT, type SourceHealth } from '../api/tauri-bridge';

const HEALTHY: SourceHealth = { outOfSyncTargets: [], git: null };

// Module-level store fed by the Rust source health check (source changes, then every 15 min).
let state = HEALTHY;
let started = false;
const listeners = new Set<() => void>();

function publish(next: SourceHealth) {
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
  void listen<SourceHealth>(SOURCE_HEALTH_EVENT, (e) => publish(e.payload));
  tauriBridge
    .getSourceHealth()
    .then(publish)
    .catch(() => {});
}

/** Local-only skills, out-of-sync targets and git remote state of the active source. */
export function useSourceHealth() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => state);
}
