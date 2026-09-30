import { useState, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  tauriBridge,
  CLI_INSTALL_OUTPUT_EVENT,
  type InstallOutput,
  type InstallPlatform,
} from '../api/tauri-bridge';

export type InstallPhase = 'idle' | 'installing' | 'verified' | 'failed';

export interface InstallLine {
  id: number;
  stream: 'stdout' | 'stderr' | 'info';
  text: string;
}

const MAX_LINES = 500;

interface CliManagerState {
  cliPath: string | null;
  downloading: boolean;
  error: string | null;
  platform: InstallPlatform | null;
  installPhase: InstallPhase;
  installVersion: string | null;
  lines: InstallLine[];
}

export function useCliManager() {
  const [state, setState] = useState<CliManagerState>({
    cliPath: null,
    downloading: false,
    error: null,
    platform: null,
    installPhase: 'idle',
    installVersion: null,
    lines: [],
  });
  const lineId = useRef(0);
  const cancelled = useRef(false);

  const detect = useCallback(async () => {
    setState((s) => ({ ...s, error: null }));
    try {
      const path = await tauriBridge.detectCli();
      setState((s) => ({ ...s, cliPath: path }));
      return path;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setState((s) => ({ ...s, error: msg }));
      return null;
    }
  }, []);

  const download = useCallback(async () => {
    setState((s) => ({ ...s, downloading: true, error: null }));
    try {
      const path = await tauriBridge.downloadCli();
      setState((s) => ({ ...s, cliPath: path, downloading: false }));
      return path;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setState((s) => ({ ...s, downloading: false, error: msg }));
      return null;
    }
  }, []);

  const loadPlatform = useCallback(async () => {
    try {
      const platform = await tauriBridge.detectInstallPlatform();
      setState((s) => ({ ...s, platform }));
      return platform;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setState((s) => ({ ...s, error: `Could not detect the platform: ${msg}` }));
      return null;
    }
  }, []);

  const pushLine = useCallback((stream: InstallLine['stream'], text: string) => {
    lineId.current += 1;
    const line = { id: lineId.current, stream, text };
    setState((s) => ({ ...s, lines: [...s.lines, line].slice(-MAX_LINES) }));
  }, []);

  /** Run an install method, stream its output, and verify `skillshare version`. */
  const install = useCallback(
    async (method: string, command: string | null) => {
      cancelled.current = false;
      lineId.current = 0;
      setState((s) => ({
        ...s,
        error: null,
        installPhase: 'installing',
        installVersion: null,
        lines: [],
      }));
      pushLine('info', command ? `$ ${command}` : '$ download latest skillshare release');

      let unlisten: (() => void) | null = null;
      try {
        unlisten = await listen<InstallOutput>(CLI_INSTALL_OUTPUT_EVENT, (e) =>
          pushLine(e.payload.stream, e.payload.line)
        );
      } catch (err) {
        pushLine('info', `Live output unavailable: ${err instanceof Error ? err.message : err}`);
      }

      try {
        let path: string;
        let version: string;
        if (method === 'app-copy') {
          path = await tauriBridge.downloadCli();
          version = await tauriBridge.getCliVersion(path);
        } else {
          ({ path, version } = await tauriBridge.installCli(method));
        }
        if (cancelled.current) return null;
        pushLine('info', `Verified: skillshare ${version}`);
        setState((s) => ({
          ...s,
          cliPath: path,
          installPhase: 'verified',
          installVersion: version,
        }));
        return path;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (cancelled.current || msg === 'Installation cancelled') {
          setState((s) => ({ ...s, installPhase: 'idle' }));
        } else {
          setState((s) => ({ ...s, installPhase: 'failed', error: msg }));
        }
        return null;
      } finally {
        unlisten?.();
      }
    },
    [pushLine]
  );

  const cancelInstall = useCallback(async () => {
    cancelled.current = true;
    try {
      await tauriBridge.cancelCliInstall();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setState((s) => ({ ...s, error: `Could not cancel the installer: ${msg}` }));
    }
    setState((s) => ({ ...s, installPhase: 'idle' }));
  }, []);

  const resetInstall = useCallback(() => {
    setState((s) => ({ ...s, installPhase: 'idle', error: null, lines: [] }));
  }, []);

  return {
    cliPath: state.cliPath,
    downloading: state.downloading,
    error: state.error,
    platform: state.platform,
    installPhase: state.installPhase,
    installVersion: state.installVersion,
    lines: state.lines,
    detect,
    download,
    loadPlatform,
    install,
    cancelInstall,
    resetInstall,
  };
}
