import { useEffect, useSyncExternalStore } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { tauriBridge } from '../api/tauri-bridge';
import { currentUpdatePolicy, useUpdatePolicy } from './useUpdatePolicy';

export type UpdateCheckStatus = 'idle' | 'checking' | 'available' | 'up-to-date' | 'error';
/** The steps of an install, in order: download, install, then the automatic restart. */
export const INSTALL_STEPS = ['downloading', 'installing', 'restarting'] as const;
export type InstallStep = (typeof INSTALL_STEPS)[number];
/** The steps an install can fail in; a failed restart is its own phase. */
export type FailableStep = Exclude<InstallStep, 'restarting'>;
/** Where installing the available update is: one of the steps, or not running. */
export type InstallPhase = 'idle' | InstallStep | 'restart-failed' | 'error';

/** Position of the phase among the install steps, or -1 when no step is running. */
export function installStepIndex(phase: InstallPhase) {
  return (INSTALL_STEPS as readonly InstallPhase[]).indexOf(phase);
}

export function isInstallStep(phase: InstallPhase) {
  return installStepIndex(phase) >= 0;
}

/** The install failed, or the app could not restart itself. */
export function isInstallStopped(phase: InstallPhase) {
  return phase === 'error' || phase === 'restart-failed';
}

interface UpdateState {
  status: UpdateCheckStatus;
  update: Update | null;
  error: string | null;
  phase: InstallPhase;
  downloaded: number;
  /** Download size in bytes; 0 when the server did not send one. */
  total: number;
  installError: string | null;
  /** When the current phase started, for the elapsed time of steps without a percentage. */
  phaseAt: number;
  /** The step that was running when the install failed. */
  failedPhase: FailableStep | null;
}

const INITIAL: UpdateState = {
  status: 'idle',
  update: null,
  error: null,
  phase: 'idle',
  downloaded: 0,
  total: 0,
  installError: null,
  phaseAt: 0,
  failedPhase: null,
};

/** localStorage key for the update being applied, read after the restart to say what changed. */
export const JUST_UPDATED_KEY = 'skillshare-just-updated';
export interface JustUpdated {
  version: string;
  notes: string;
}

// Module-level store: the update check runs at most once per app session and the
// resulting Update object is shared by the settings nav badge and AboutSettings.
let state = INITIAL;
let inflight: Promise<void> | null = null;
let inflightManual = false;
let autoChecked = false;
let manualResult = false;
const listeners = new Set<() => void>();

function setState(next: Partial<UpdateState>) {
  const phaseAt = next.phase && next.phase !== state.phase ? Date.now() : state.phaseAt;
  state = { ...state, phaseAt, ...next };
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function runCheck(manual = true): Promise<void> {
  if (inflight) {
    inflightManual ||= manual;
    return inflight;
  }
  inflightManual = manual;
  setState({ status: 'checking', error: null });
  inflight = (async () => {
    try {
      const update = await check();
      if (!inflightManual && !currentUpdatePolicy()?.enabled) {
        setState({ status: 'idle', update: null });
        return;
      }
      manualResult = inflightManual;
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

/** Forget a failed install or a restart that did not happen, so the row offers the update again. */
export function clearAppUpdateResult() {
  if (state.phase === 'error' || state.phase === 'restart-failed')
    setState({ phase: 'idle', downloaded: 0, total: 0, installError: null, failedPhase: null });
}

/** Download and install the available update, then restart into it. */
async function install() {
  const update = state.update;
  if (!update || (state.phase !== 'idle' && state.phase !== 'error')) return;
  setState({
    phase: 'downloading',
    downloaded: 0,
    total: 0,
    installError: null,
    failedPhase: null,
  });
  let at: FailableStep = 'downloading';
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') setState({ total: event.data.contentLength ?? 0 });
      if (event.event === 'Progress')
        setState({ downloaded: state.downloaded + event.data.chunkLength });
      if (event.event === 'Finished') {
        at = 'installing';
        setState({ phase: 'installing' });
      }
    });
  } catch (err) {
    setState({
      phase: 'error',
      failedPhase: at,
      installError: err instanceof Error ? err.message : String(err),
    });
    return;
  }
  try {
    const note: JustUpdated = { version: update.version, notes: update.body ?? '' };
    localStorage.setItem(JUST_UPDATED_KEY, JSON.stringify(note));
  } catch {
    // Storage can be unavailable; the update still applies, only the notice is lost.
  }
  setState({ phase: 'restarting' });
  try {
    await tauriBridge.relaunchApp();
  } catch {
    setState({ phase: 'restart-failed' });
  }
}

export function useAppUpdate() {
  const s = useSyncExternalStore(subscribe, () => state);
  const policy = useUpdatePolicy();

  useEffect(() => {
    // The browser preview has no updater backend; leave it idle until asked.
    if (autoChecked || !isTauri() || !policy?.enabled) return;
    autoChecked = true;
    void runCheck(false);
  }, [policy?.enabled]);

  return {
    available: policy?.enabled === true && s.status === 'available',
    version: policy?.enabled || manualResult ? (s.update?.version ?? null) : null,
    update: s.update,
    status: s.status,
    error: s.error,
    recheck: runCheck,
    phase: s.phase,
    downloaded: s.downloaded,
    total: s.total,
    installError: s.installError,
    phaseAt: s.phaseAt,
    failedPhase: s.failedPhase,
    install,
    restart: () => tauriBridge.relaunchApp().catch(() => setState({ phase: 'restart-failed' })),
  };
}

export function resetAppUpdateForTests() {
  state = INITIAL;
  inflight = null;
  autoChecked = false;
  manualResult = false;
}
