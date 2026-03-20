import { useState } from 'react';
import { Folder, Globe, Plus, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';
import Button from '../../../components/Button';
import { useProjects } from '../../context/ProjectContext';
import { tauriBridge } from '../../api/tauri-bridge';

export default function ProjectSettings() {
  const { projects, activeProject, addProject, removeProject, switchWithRestart } = useProjects();
  const navigate = useNavigate();
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasGlobal = projects.some((p) => p.projectType === 'global');

  const handleAddGlobal = async () => {
    setAdding(true);
    setError(null);
    try {
      const cliPath = await tauriBridge.detectCli();
      if (!cliPath) throw new Error('CLI not found');
      const home = await homeDir();
      try {
        await tauriBridge.runCli(cliPath, ['init'], home);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (!msg.toLowerCase().includes('already initialized')) throw err;
      }
      const configDir = await tauriBridge.getGlobalConfigDir(cliPath);
      await addProject('Global', configDir || home, 'global');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setAdding(false);
    }
  };

  const handleAddProject = async () => {
    const dir = await open({ directory: true, title: 'Select project directory' });
    if (typeof dir !== 'string') return;
    setAdding(true);
    setError(null);
    try {
      const cliPath = await tauriBridge.detectCli();
      if (!cliPath) throw new Error('CLI not found');
      try {
        await tauriBridge.runCli(cliPath, ['init', '-p'], dir);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (!msg.toLowerCase().includes('already initialized')) throw err;
      }
      const name = dir.split('/').pop() || 'Project';
      await addProject(name, dir, 'project');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setAdding(false);
    }
  };

  const handleRemove = async (id: string) => {
    try {
      await removeProject(id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSwitch = async (id: string) => {
    await switchWithRestart(id);
    navigate('/');
  };

  return (
    <div className="space-y-6">
      <div>
        <h1
          className="text-2xl font-bold text-pencil"
          style={{ fontFamily: 'var(--font-heading)' }}
        >
          Projects
        </h1>
        <p className="text-sm text-pencil-light mt-1">
          Manage your skillshare projects and global configuration.
        </p>
      </div>

      {error && <p className="text-danger text-sm">{error}</p>}

      {/* Project list header */}
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-pencil-light uppercase tracking-wider">
          Select Project
        </h2>
        <div className="flex gap-2">
          {!hasGlobal && (
            <Button variant="secondary" size="sm" onClick={handleAddGlobal} loading={adding}>
              Add Global
            </Button>
          )}
          <Button size="sm" onClick={handleAddProject} loading={adding}>
            Add Project
          </Button>
        </div>
      </div>

      {/* Project cards — Codex style */}
      <div className="space-y-2">
        {projects.map((project) => {
          const isActive = project.id === activeProject?.id;
          const isGlobal = project.projectType === 'global';

          return (
            <div
              key={project.id}
              className={`flex items-center justify-between gap-3 px-4 py-3 rounded-[var(--radius-md)] border transition-colors ${
                isActive
                  ? 'border-pencil bg-muted/20'
                  : 'border-muted hover:border-pencil-light cursor-pointer'
              }`}
              onClick={!isActive && !isGlobal ? () => handleSwitch(project.id) : undefined}
              role={!isActive && !isGlobal ? 'button' : undefined}
            >
              <div className="flex items-center gap-3 min-w-0">
                {isGlobal ? (
                  <Globe size={16} strokeWidth={2.5} className="shrink-0 text-pencil-light" />
                ) : (
                  <Folder size={16} strokeWidth={2.5} className="shrink-0 text-pencil-light" />
                )}
                <span className="font-medium text-pencil truncate">{project.name}</span>
                <span className="text-sm text-pencil-light truncate">{project.path}</span>
              </div>

              <div className="flex items-center gap-2 shrink-0">
                {isActive && <span className="text-xs text-pencil-light">Active</span>}
                {!isGlobal && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemove(project.id);
                    }}
                    className="p-1 rounded-[var(--radius-sm)] text-pencil-light hover:text-danger hover:bg-muted/30 transition-colors"
                    title="Remove project"
                  >
                    <Trash2 size={14} strokeWidth={2.5} />
                  </button>
                )}
                {!isActive && !isGlobal && (
                  <Plus size={16} strokeWidth={2.5} className="text-pencil-light" />
                )}
              </div>
            </div>
          );
        })}

        {projects.length === 0 && (
          <div className="text-center py-12 text-pencil-light text-sm">
            No projects yet. Add one to get started.
          </div>
        )}
      </div>
    </div>
  );
}
