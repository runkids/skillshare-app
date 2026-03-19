import { useState, useEffect, useCallback, useRef } from 'react';
import { tauriBridge, type Project } from '../api/tauri-bridge';

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeProject, setActiveProject] = useState<Project | null>(null);
  const [switching, setSwitching] = useState(false);
  const switchLock = useRef(false);

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

  const removeProject = useCallback(
    async (id: string) => {
      await tauriBridge.removeProject(id);
      await refresh();
    },
    [refresh]
  );

  const switchWithRestart = useCallback(
    async (id: string) => {
      if (switchLock.current) return;
      switchLock.current = true;
      setSwitching(true);
      try {
        await tauriBridge.stopServer();
        await tauriBridge.switchProject(id);
        await refresh();
        const cliPath = await tauriBridge.detectCli();
        const active = await tauriBridge.getActiveProject();
        const projectDir = active?.path;
        const port = await tauriBridge.startServer(cliPath!, projectDir);
        return port;
      } finally {
        setSwitching(false);
        switchLock.current = false;
      }
    },
    [refresh]
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    projects,
    activeProject,
    switching,
    refresh,
    addProject,
    switchProject,
    switchWithRestart,
    removeProject,
  };
}
