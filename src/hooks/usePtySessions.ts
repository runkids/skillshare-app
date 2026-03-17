/**
 * PTY Sessions Hook - Manages PTY terminal session state
 * Extracted from ScriptPtyTerminal for better separation of concerns
 */

import { useState, useCallback, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebglAddon } from '@xterm/addon-webgl';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { IPty } from 'tauri-pty';

// PTY session info
export interface PtySession {
  id: string;
  name: string;
  projectPath: string;
  projectName?: string;
  pty: IPty | null;
  terminal: Terminal | null;
  fitAddon: FitAddon | null;
  searchAddon: SearchAddon | null;
  webglAddon: WebglAddon | null;
  status: 'running' | 'completed' | 'failed';
  exitCode?: number;
}

// Terminal configuration for better readability
export const terminalTheme = {
  background: '#030712', // gray-950
  foreground: '#e5e7eb', // gray-200 (improved contrast)
  cursor: '#e5e7eb',
  cursorAccent: '#030712',
  selectionBackground: 'rgba(59, 130, 246, 0.4)', // blue-500 with opacity
  selectionForeground: '#ffffff',
  black: '#374151',
  red: '#ef4444',
  green: '#22c55e',
  yellow: '#eab308',
  blue: '#3b82f6',
  magenta: '#a855f7',
  cyan: '#06b6d4',
  white: '#e5e7eb', // gray-200
  brightBlack: '#9ca3af', // gray-400 (brighter for better visibility)
  brightRed: '#f87171',
  brightGreen: '#4ade80',
  brightYellow: '#facc15',
  brightBlue: '#60a5fa',
  brightMagenta: '#c084fc',
  brightCyan: '#22d3ee',
  brightWhite: '#f9fafb', // gray-50
};

export const terminalOptions = {
  cursorBlink: true,
  cursorStyle: 'block' as const,
  fontSize: 14,
  fontFamily:
    '"SF Mono", Menlo, Monaco, "Cascadia Code", "Fira Code", Consolas, "Courier New", monospace',
  fontWeight: '400' as const,
  fontWeightBold: '600' as const,
  lineHeight: 1.5,
  letterSpacing: 0.5,
  theme: terminalTheme,
  scrollback: 5000,
  allowProposedApi: true,
  allowTransparency: false,
  minimumContrastRatio: 4.5,
};

interface UsePtySessionsOptions {
  onRegisterPtyExecution?: (
    sessionId: string,
    scriptName: string,
    projectPath: string,
    projectName?: string
  ) => void;
  onRemovePtyExecution?: (sessionId: string) => void;
}

interface UsePtySessionsReturn {
  sessions: Map<string, PtySession>;
  activeSessionId: string | null;
  activeSession: PtySession | null;
  sessionList: PtySession[];
  setActiveSessionId: (id: string | null) => void;
  setSessions: React.Dispatch<React.SetStateAction<Map<string, PtySession>>>;
  spawnSession: (
    command: string,
    args: string[],
    cwd: string,
    name: string,
    projectName?: string
  ) => Promise<string | null>;
  killSession: (sessionId: string) => void;
  killAllSessions: () => void;
  pendingSpawnsRef: React.MutableRefObject<
    Map<string, { command: string; args: string[]; cwd: string }>
  >;
}

export function usePtySessions({
  onRegisterPtyExecution,
  onRemovePtyExecution,
}: UsePtySessionsOptions = {}): UsePtySessionsReturn {
  const [sessions, setSessions] = useState<Map<string, PtySession>>(new Map());
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  // Track pending spawns that need PTY initialization after terminal mount
  const pendingSpawnsRef = useRef<Map<string, { command: string; args: string[]; cwd: string }>>(
    new Map()
  );

  // Get active session
  const activeSession = activeSessionId ? (sessions.get(activeSessionId) ?? null) : null;

  // Session list for iteration
  const sessionList = Array.from(sessions.values());

  // Spawn a new PTY session
  const spawnSession = useCallback(
    async (
      command: string,
      args: string[],
      cwd: string,
      name: string,
      projectName?: string
    ): Promise<string | null> => {
      console.log('[PTY] spawnSession called:', { command, args, cwd, name, projectName });
      const sessionId = crypto.randomUUID();

      // Create terminal instance with improved readability settings
      const term = new Terminal(terminalOptions);

      const fitAddon = new FitAddon();
      const searchAddon = new SearchAddon();
      const webLinksAddon = new WebLinksAddon((_event, uri) => {
        // Open URL in default browser using Tauri opener plugin
        openUrl(uri).catch((err) => console.error('[PTY] Failed to open URL:', err));
      });
      term.loadAddon(fitAddon);
      term.loadAddon(searchAddon);
      term.loadAddon(webLinksAddon);

      // Create initial session
      const session: PtySession = {
        id: sessionId,
        name,
        projectPath: cwd,
        projectName,
        pty: null,
        terminal: term,
        fitAddon,
        searchAddon,
        webglAddon: null,
        status: 'running',
      };

      // Store spawn info for later initialization
      pendingSpawnsRef.current.set(sessionId, { command, args, cwd });

      setSessions((prev) => {
        const next = new Map(prev);
        next.set(sessionId, session);
        return next;
      });
      setActiveSessionId(sessionId);

      // Register with ScriptExecutionContext for icon state and port detection
      onRegisterPtyExecution?.(sessionId, name, cwd, projectName);

      return sessionId;
    },
    [onRegisterPtyExecution]
  );

  // Kill a session
  const killSession = useCallback(
    (sessionId: string) => {
      const session = sessions.get(sessionId);
      if (session) {
        session.pty?.kill();
        session.webglAddon?.dispose();
        session.terminal?.dispose();
        setSessions((prev) => {
          const next = new Map(prev);
          next.delete(sessionId);
          // Select another session if active was removed
          if (activeSessionId === sessionId) {
            const remaining = Array.from(next.keys());
            setActiveSessionId(remaining.length > 0 ? remaining[remaining.length - 1] : null);
          }
          return next;
        });
        // Remove from ScriptExecutionContext
        onRemovePtyExecution?.(sessionId);
      }
    },
    [sessions, activeSessionId, onRemovePtyExecution]
  );

  // Kill all running sessions
  const killAllSessions = useCallback(() => {
    sessions.forEach((session) => {
      if (session.status === 'running') {
        session.pty?.kill();
        session.webglAddon?.dispose();
        session.terminal?.dispose();
        onRemovePtyExecution?.(session.id);
      }
    });
    setSessions(new Map());
    setActiveSessionId(null);
  }, [sessions, onRemovePtyExecution]);

  return {
    sessions,
    activeSessionId,
    activeSession,
    sessionList,
    setActiveSessionId,
    setSessions,
    spawnSession,
    killSession,
    killAllSessions,
    pendingSpawnsRef,
  };
}
