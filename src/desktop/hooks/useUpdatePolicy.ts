import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, type UpdatePolicy } from '../api/tauri-bridge';

let policy: UpdatePolicy | null = null;
let started = false;
const listeners = new Set<() => void>();

function publish(next: UpdatePolicy) {
  policy = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function currentUpdatePolicy() {
  return policy;
}

export async function refreshUpdatePolicy() {
  try {
    const next = await tauriBridge.getUpdatePolicy();
    publish(next);
    return next;
  } catch (err) {
    const next: UpdatePolicy = {
      enabled: false,
      configured: true,
      envOverride: false,
      supported: false,
      error: String(err),
    };
    publish(next);
    return next;
  }
}

export async function setUpdatePolicy(enabled: boolean) {
  const next = await tauriBridge.setUpdatePolicy(enabled);
  publish(next);
}

function start() {
  if (started || !isTauri()) return;
  started = true;
  void listen<UpdatePolicy>('update-policy-changed', (e) => publish(e.payload));
  void refreshUpdatePolicy().catch(() => {});
}

export function useUpdatePolicy() {
  useEffect(start, []);
  return useSyncExternalStore(subscribe, () => policy);
}

export function resetUpdatePolicyForTests() {
  policy = null;
  started = false;
}
