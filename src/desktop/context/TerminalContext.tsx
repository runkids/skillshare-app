// src/desktop/context/TerminalContext.tsx
import { createContext, useContext, useState, useRef, useCallback, useEffect } from 'react';
import type { ReactNode } from 'react';
import type { IPty } from 'tauri-pty';
import { useProjects } from './ProjectContext';
import {
  createTerminalInstance,
  mountTerminal,
  createBufferedWriter,
  disposeTerminalInstance,
  type TerminalInstanceHandle,
  type BufferedWriter,
} from '../hooks/useTerminalInstance';
import { spawnPty, killPty, writeToPty, resizePty } from '../hooks/usePtySpawn';

// --- State types (React state, drives UI) ---

export interface TerminalSessionState {
  id: string;
  projectId: string;
  projectPath: string;
  name: string;
  status: 'running' | 'completed' | 'failed';
  exitCode?: number;
  hasUnread: boolean;
}

// --- Live object types (ref store, NOT in React state) ---

interface TerminalSessionLive {
  pty: IPty | null;
  handle: TerminalInstanceHandle | null;
  bufferedWriter: BufferedWriter | null;
  cleanupMount: (() => void) | null;
  projectPath: string; // cached here to avoid stale sessionStore closures
}

// --- Context interface ---

interface TerminalContextValue {
  sessions: TerminalSessionState[];
  activeSessionId: string | null;
  activeView: 'webui' | 'terminal';
  setActiveView: (view: 'webui' | 'terminal') => void;
  spawnSession: (command?: string, args?: string[], name?: string) => Promise<string>;
  killSession: (id: string) => void;
  switchSession: (id: string) => void;
  mountSession: (id: string, container: HTMLElement) => void;
  unmountSession: (id: string) => void;
  executeInSession: (id: string, command: string) => void;
  hasUnreadAny: boolean;
  hasUnreadByProject: Map<string, boolean>;
  killProjectSessions: (projectId: string) => void;
  getSessionLive: (id: string) => TerminalSessionLive | undefined;
}

const TerminalContext = createContext<TerminalContextValue>(null!);

export function useTerminal() {
  return useContext(TerminalContext);
}

export function TerminalProvider({ children }: { children: ReactNode }) {
  const { activeProject, registerOnProjectRemoved } = useProjects();

  // React state: grouped by project
  const [sessionStore, setSessionStore] = useState<Map<string, TerminalSessionState[]>>(new Map());
  const [activeSessionIds, setActiveSessionIds] = useState<Map<string, string>>(new Map());
  const [activeView, setActiveView] = useState<'webui' | 'terminal'>('webui');

  // Ref store: live objects keyed by sessionId
  const liveStoreRef = useRef<Map<string, TerminalSessionLive>>(new Map());

  const currentProjectId = activeProject?.id ?? 'global';
  const currentProjectPath = activeProject?.path ?? '~';

  // Current project's sessions
  const sessions = sessionStore.get(currentProjectId) ?? [];
  const activeSessionId = activeSessionIds.get(currentProjectId) ?? null;

  // Unread tracking
  const hasUnreadByProject = new Map<string, boolean>();
  for (const [pid, ss] of sessionStore) {
    hasUnreadByProject.set(
      pid,
      ss.some((s) => s.hasUnread)
    );
  }
  const hasUnreadAny = Array.from(hasUnreadByProject.values()).some(Boolean);

  // --- Helpers ---

  const updateSession = useCallback((sessionId: string, updates: Partial<TerminalSessionState>) => {
    setSessionStore((prev) => {
      const next = new Map(prev);
      for (const [pid, ss] of next) {
        const idx = ss.findIndex((s) => s.id === sessionId);
        if (idx !== -1) {
          const updated = [...ss];
          updated[idx] = { ...updated[idx], ...updates };
          next.set(pid, updated);
          return next;
        }
      }
      return prev;
    });
  }, []);

  const isSessionVisible = useCallback(
    (sessionId: string) => {
      return activeView === 'terminal' && activeSessionId === sessionId;
    },
    [activeView, activeSessionId]
  );

  // --- API ---

  const spawnSession = useCallback(
    async (command?: string, args?: string[], name?: string) => {
      const id = crypto.randomUUID();
      const sessionName =
        name ??
        (command
          ? command.split(' ').pop()!
          : `Shell ${(sessionStore.get(currentProjectId)?.length ?? 0) + 1}`);

      const state: TerminalSessionState = {
        id,
        projectId: currentProjectId,
        projectPath: currentProjectPath,
        name: sessionName,
        status: 'running',
        hasUnread: false,
      };

      // Create terminal instance
      const handle = createTerminalInstance({
        onData: (data) => {
          const live = liveStoreRef.current.get(id);
          if (live?.pty) {
            writeToPty(live.pty, data);
          }
        },
      });

      const bufferedWriter = createBufferedWriter(handle.terminal);

      const live: TerminalSessionLive = {
        pty: null,
        handle,
        bufferedWriter,
        cleanupMount: null,
        projectPath: currentProjectPath,
      };
      liveStoreRef.current.set(id, live);

      // Add to state
      setSessionStore((prev) => {
        const next = new Map(prev);
        const existing = next.get(currentProjectId) ?? [];
        next.set(currentProjectId, [...existing, state]);
        return next;
      });
      setActiveSessionIds((prev) => new Map(prev).set(currentProjectId, id));

      return id;
    },
    [currentProjectId, currentProjectPath, sessionStore]
  );

  const mountSession = useCallback(
    (id: string, container: HTMLElement) => {
      const live = liveStoreRef.current.get(id);
      if (!live?.handle) return;

      // Mount xterm to DOM
      live.cleanupMount = mountTerminal(live.handle, container);

      // Spawn PTY after terminal is mounted (need cols/rows)
      const { terminal } = live.handle;
      const cols = terminal.cols;
      const rows = terminal.rows;

      // Only spawn PTY if not already spawned
      if (live.pty) return;

      // Use projectPath from live store (avoids stale sessionStore closure)
      spawnPty({
        cwd: live.projectPath,
        cols,
        rows,
        onData: (data) => {
          live.bufferedWriter?.write(data);
          // Mark unread if not visible
          if (!isSessionVisible(id)) {
            updateSession(id, { hasUnread: true });
          }
        },
        onExit: (exitCode) => {
          updateSession(id, {
            status: exitCode === 0 ? 'completed' : 'failed',
            exitCode,
          });
        },
      }).then((pty) => {
        live.pty = pty;

        // Sync resize: when terminal resizes, resize PTY
        terminal.onResize(({ cols, rows }) => {
          resizePty(pty, cols, rows);
        });
      });
    },
    [isSessionVisible, updateSession]
  );

  const unmountSession = useCallback((id: string) => {
    const live = liveStoreRef.current.get(id);
    if (live?.cleanupMount) {
      live.cleanupMount();
      live.cleanupMount = null;
    }
  }, []);

  const killSession = useCallback((id: string) => {
    const live = liveStoreRef.current.get(id);
    if (live) {
      live.bufferedWriter?.dispose();
      if (live.pty) killPty(live.pty);
      if (live.handle) disposeTerminalInstance(live.handle);
      live.cleanupMount?.();
      liveStoreRef.current.delete(id);
    }

    setSessionStore((prev) => {
      const next = new Map(prev);
      for (const [pid, ss] of next) {
        const filtered = ss.filter((s) => s.id !== id);
        if (filtered.length !== ss.length) {
          next.set(pid, filtered);
          // Update active session from the freshly filtered list
          setActiveSessionIds((p) => {
            const n = new Map(p);
            if (n.get(pid) === id) {
              if (filtered.length > 0) {
                n.set(pid, filtered[filtered.length - 1].id);
              } else {
                n.delete(pid);
              }
            }
            return n;
          });
        }
      }
      return next;
    });
  }, []);

  const switchSession = useCallback(
    (id: string) => {
      setActiveSessionIds((prev) => new Map(prev).set(currentProjectId, id));
      updateSession(id, { hasUnread: false });
    },
    [currentProjectId, updateSession]
  );

  const executeInSession = useCallback((id: string, command: string) => {
    const live = liveStoreRef.current.get(id);
    if (live?.pty) {
      writeToPty(live.pty, command + '\r');
    }
  }, []);

  const killProjectSessions = useCallback(
    (projectId: string) => {
      const projectSessions = sessionStore.get(projectId) ?? [];
      for (const s of projectSessions) {
        const live = liveStoreRef.current.get(s.id);
        if (live) {
          live.bufferedWriter?.dispose();
          if (live.pty) killPty(live.pty);
          if (live.handle) disposeTerminalInstance(live.handle);
          live.cleanupMount?.();
          liveStoreRef.current.delete(s.id);
        }
      }
      setSessionStore((prev) => {
        const next = new Map(prev);
        next.delete(projectId);
        return next;
      });
      setActiveSessionIds((prev) => {
        const next = new Map(prev);
        next.delete(projectId);
        return next;
      });
    },
    [sessionStore]
  );

  const getSessionLive = useCallback((id: string) => {
    return liveStoreRef.current.get(id);
  }, []);

  // Kill all sessions for a project when it is removed
  useEffect(() => {
    return registerOnProjectRemoved((projectId) => {
      killProjectSessions(projectId);
    });
  }, [registerOnProjectRemoved, killProjectSessions]);

  // Clear hasUnread when switching to a session
  useEffect(() => {
    if (activeSessionId && activeView === 'terminal') {
      updateSession(activeSessionId, { hasUnread: false });
    }
  }, [activeSessionId, activeView, updateSession]);

  return (
    <TerminalContext.Provider
      value={{
        sessions,
        activeSessionId,
        activeView,
        setActiveView,
        spawnSession,
        killSession,
        switchSession,
        mountSession,
        unmountSession,
        executeInSession,
        hasUnreadAny,
        hasUnreadByProject,
        killProjectSessions,
        getSessionLive,
      }}
    >
      {children}
    </TerminalContext.Provider>
  );
}
