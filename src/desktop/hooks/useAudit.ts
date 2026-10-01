import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, AUDIT_EVENT, type AuditFinding } from '../api/tauri-bridge';

const NONE: AuditFinding[] = [];

// Module-level store fed by the Rust audit that runs after Update All, Pull and auto-sync.
let state = NONE;
let started = false;
const listeners = new Set<() => void>();

function publish(next: AuditFinding[]) {
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
  void listen<AuditFinding[]>(AUDIT_EVENT, (e) => publish(e.payload));
  tauriBridge
    .getAuditReport()
    .then(publish)
    .catch(() => {});
}

/** Security audit findings for the active project from the last post-change audit. */
export function useAudit() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => state);
}
