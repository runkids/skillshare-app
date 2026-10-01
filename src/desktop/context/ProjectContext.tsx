import { createContext, useContext, useState, useEffect, useCallback, useRef } from 'react';
import type { ReactNode, RefObject } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { tauriBridge, TRAY_PROJECT_REQUESTED_EVENT, type Project } from '../api/tauri-bridge';

export interface SwitchOptions {
  newSession?: boolean;
}

interface ProjectContextValue {
  projects: Project[];
  activeProject: Project | null;
  switching: boolean;
  refresh: () => Promise<void>;
  addProject: (name: string, path: string, projectType: 'global' | 'project') => Promise<Project>;
  switchProject: (id: string) => Promise<void>;
  switchWithRestart: (id: string, options?: SwitchOptions) => Promise<number | undefined>;
  removeProject: (id: string) => Promise<void>;
  registerOnProjectRemoved: (callback: (projectId: string) => void) => () => void;
  lastSwitchOptionsRef: RefObject<SwitchOptions | null>;
  /** Increments on each reloadView() call; the web view reloads when it changes. */
  reloadKey: number;
  reloadView: () => void;
}

const ProjectContext = createContext<ProjectContextValue>({
  projects: [],
  activeProject: null,
  switching: false,
  refresh: async () => {},
  addProject: async () => ({}) as Project,
  switchProject: async () => {},
  switchWithRestart: async () => undefined,
  removeProject: async () => {},
  registerOnProjectRemoved: () => () => {},
  lastSwitchOptionsRef: { current: null },
  reloadKey: 0,
  reloadView: () => {},
});

// eslint-disable-next-line react-refresh/only-export-components -- hook lives beside its provider
export function useProjects() {
  return useContext(ProjectContext);
}

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeProject, setActiveProject] = useState<Project | null>(null);
  const [switching, setSwitching] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const reloadView = useCallback(() => setReloadKey((k) => k + 1), []);
  const switchLock = useRef(false);
  const lastSwitchOptionsRef = useRef<SwitchOptions | null>(null);
  const projectRemovedCallbacks = useRef<Set<(projectId: string) => void>>(new Set());

  const registerOnProjectRemoved = useCallback((callback: (projectId: string) => void) => {
    projectRemovedCallbacks.current.add(callback);
    return () => {
      projectRemovedCallbacks.current.delete(callback);
    };
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [list, active] = await Promise.all([
        tauriBridge.listProjects(),
        tauriBridge.getActiveProject(),
      ]);
      setProjects(list);
      setActiveProject(active);
    } catch {
      // Silently ignore — projects may not exist yet
    }
  }, []);

  const addProject = useCallback(
    async (name: string, path: string, projectType: 'global' | 'project') => {
      const project = await tauriBridge.addProject(name, path, projectType);
      await refresh();
      return project;
    },
    [refresh]
  );

  const switchProject = useCallback(
    async (id: string) => {
      setSwitching(true);
      try {
        await tauriBridge.switchProject(id);
        await refresh();
      } finally {
        setSwitching(false);
      }
    },
    [refresh]
  );

  const switchWithRestart = useCallback(
    async (id: string, options?: SwitchOptions) => {
      if (switchLock.current) return undefined;
      lastSwitchOptionsRef.current = options ?? null;
      switchLock.current = true;
      setSwitching(true);
      try {
        await tauriBridge.stopServer();
        await tauriBridge.switchProject(id);
        const [, cliPath] = await Promise.all([refresh(), tauriBridge.detectCli()]);
        if (!cliPath) throw new Error('CLI not found');
        // activeProject is updated by refresh(); read from store for the dir
        const active = await tauriBridge.getActiveProject();
        const port = await tauriBridge.startServer(cliPath, active?.path);
        return port;
      } finally {
        setSwitching(false);
        switchLock.current = false;
      }
    },
    [refresh]
  );

  // Tray and in-app project choices use the same lock, state and server lifecycle.
  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<string>(TRAY_PROJECT_REQUESTED_EVENT, (event) => {
      if (event.payload === activeProject?.id) return;
      void switchWithRestart(event.payload).catch((error: unknown) => {
        console.error('Tray project switch failed', error);
      });
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [activeProject?.id, switchWithRestart]);

  const removeProject = useCallback(
    async (id: string) => {
      const wasActive = (await tauriBridge.getActiveProject())?.id === id;
      await tauriBridge.removeProject(id);
      await refresh();
      for (const cb of projectRemovedCallbacks.current) {
        cb(id);
      }
      if (!wasActive) return;
      // The store promotes another project to active, but the running server still
      // serves the removed one's directory. Restart it so the web view follows.
      const next = await tauriBridge.getActiveProject();
      if (next) {
        await switchWithRestart(next.id);
      } else {
        await tauriBridge.stopServer();
      }
    },
    [refresh, switchWithRestart]
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  // A tray Quick Sync changed the skills on disk; show the result in the web view.
  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen('sync-completed', reloadView);
    return () => {
      void unlisten.then((off) => off());
    };
  }, [reloadView]);

  return (
    <ProjectContext.Provider
      value={{
        projects,
        activeProject,
        switching,
        refresh,
        addProject,
        switchProject,
        switchWithRestart,
        removeProject,
        registerOnProjectRemoved,
        lastSwitchOptionsRef,
        reloadKey,
        reloadView,
      }}
    >
      {children}
    </ProjectContext.Provider>
  );
}
