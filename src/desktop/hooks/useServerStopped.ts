import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Module-level store, so the title bar dot and the status panel agree on a dead server.
// `appInfo` is read once, so follow the supervisor's events to notice one.
let stopped = false;
let started = false;
const listeners = new Set<() => void>();

function publish(next: boolean) {
  stopped = next;
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function start() {
  if (started || !isTauri()) return;
  started = true;
  void listen('server-stopped', () => publish(true));
  void listen('server-restarted', () => publish(false));
}

/** Forget the stop before the header reload restarts the server. */
export function clearServerStopped() {
  publish(false);
}

export function resetServerStoppedForTests() {
  stopped = false;
  started = false;
}

/** True after the supervisor gave up on the server, until it restarts. */
export function useServerStopped() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => stopped);
}
