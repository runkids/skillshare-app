import { useState } from 'react';
import { Folder, FolderOpen, Globe, Plus, Trash2, X } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import Button from '../../../components/Button';
import { useProjects } from '../../context/ProjectContext';
import { tauriBridge, type Project } from '../../api/tauri-bridge';
import { shortPath } from '../../utils/path';
import { SettingsHeader, SettingsList, SettingsSection, StatusDot } from './SettingsLayout';

const iconButton =
  'w-8 h-8 flex items-center justify-center rounded-[var(--r-ctl)] text-[var(--ink-3)] hover:text-[var(--ink)] hover:bg-[var(--sel)] transition-colors disabled:opacity-40 disabled:pointer-events-none';

export default function ProjectSettings() {
  const { projects, activeProject, addProject, removeProject, switchWithRestart, switching } =
    useProjects();
  const navigate = useNavigate();
  const [adding, setAdding] = useState(false);
  const [switchingTo, setSwitchingTo] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const globals = projects.filter((p) => p.projectType === 'global');
  const repos = projects.filter((p) => p.projectType === 'project');

  const run = async (action: () => Promise<void>) => {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleAddProject = async () => {
    const dir = await open({ directory: true, title: 'Select project directory' });
    if (typeof dir !== 'string') return;
    setAdding(true);
    await run(async () => {
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
    });
    setAdding(false);
  };

  const handleSwitch = async (id: string) => {
    setSwitchingTo(id);
    await run(async () => {
      await switchWithRestart(id);
      navigate('/');
    });
    setSwitchingTo(null);
  };

  const handleRemove = (id: string) =>
    run(async () => {
      await removeProject(id);
      setConfirmRemove(null);
    });

  const renderRow = (project: Project) => {
    const isActive = project.id === activeProject?.id;
    const isGlobal = project.projectType === 'global';
    const Icon = isGlobal ? Globe : Folder;

    if (confirmRemove === project.id) {
      return (
        <div
          key={project.id}
          role="alert"
          className="flex items-center gap-3 px-4 py-3 bg-[var(--bad-bg)]"
        >
          <p className="flex-1 min-w-0 text-[13px] text-[var(--ink-2)]">
            <b className="text-[var(--ink)]">Remove “{project.name}” from the app?</b> The folder
            and its skills stay on disk.
          </p>
          <Button size="sm" variant="secondary" onClick={() => setConfirmRemove(null)}>
            Cancel
          </Button>
          <Button size="sm" variant="danger" onClick={() => handleRemove(project.id)}>
            Remove
          </Button>
        </div>
      );
    }

    return (
      <div key={project.id} className="flex items-center gap-3 px-4 py-3">
        <span className="w-9 h-9 shrink-0 rounded-[var(--r-ctl)] bg-[var(--accent-bg)] text-[var(--accent)] flex items-center justify-center">
          <Icon size={17} />
        </span>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-semibold text-[var(--ink)] truncate">{project.name}</p>
          <p className="font-mono text-xs text-[var(--ink-3)] truncate" title={project.path}>
            {shortPath(project.path)}
          </p>
        </div>
        {isActive ? (
          <StatusDot tone="ok">Active</StatusDot>
        ) : (
          <Button
            size="sm"
            variant="secondary"
            onClick={() => handleSwitch(project.id)}
            loading={switchingTo === project.id}
            disabled={switching}
          >
            Switch
          </Button>
        )}
        <button
          type="button"
          className={iconButton}
          onClick={() =>
            // The raw opener error (e.g. a missing desktop portal on Linux) means nothing to users.
            run(() =>
              revealItemInDir(project.path).catch(() => {
                throw new Error(`Couldn't open the folder. It's at ${project.path}`);
              })
            )
          }
          title="Show in folder"
          aria-label={`Show ${project.name} in folder`}
        >
          <FolderOpen size={16} />
        </button>
        {!isGlobal && (
          <button
            type="button"
            className={`${iconButton} hover:!text-[var(--bad)]`}
            onClick={() => setConfirmRemove(project.id)}
            disabled={switching}
            title="Remove from app"
            aria-label={`Remove ${project.name}`}
          >
            <Trash2 size={16} />
          </button>
        )}
      </div>
    );
  };

  return (
    <div className="flex flex-col gap-7">
      <SettingsHeader
        title="Projects"
        action={
          <Button size="sm" onClick={handleAddProject} loading={adding} className="shrink-0">
            <Plus size={14} /> Add project folder
          </Button>
        }
      />

      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 px-3.5 py-2.5 rounded-[var(--r-ctl)] bg-[var(--bad-bg)] text-[var(--bad)] text-[13px]"
        >
          <span className="flex-1 whitespace-pre-wrap">{error}</span>
          <button type="button" onClick={() => setError(null)} aria-label="Dismiss error">
            <X size={14} />
          </button>
        </div>
      )}

      {globals.length > 0 && (
        <SettingsSection title="Global">
          <SettingsList className="overflow-hidden">{globals.map(renderRow)}</SettingsList>
        </SettingsSection>
      )}

      <SettingsSection
        title={
          <>
            Project folders
            {repos.length > 0 && <span className="ml-1.5 font-normal">{repos.length}</span>}
          </>
        }
      >
        {repos.length > 0 ? (
          <SettingsList className="overflow-hidden">{repos.map(renderRow)}</SettingsList>
        ) : (
          <div className="flex flex-col items-center gap-3 px-6 py-10 rounded-[var(--r-box)] border border-dashed border-[var(--line-2)] text-center">
            <span className="w-10 h-10 rounded-full bg-[var(--sunken)] text-[var(--ink-3)] flex items-center justify-center">
              <Folder size={18} />
            </span>
            <div>
              <p className="text-sm font-semibold text-[var(--ink)]">No projects yet</p>
              <p className="text-[13px] text-[var(--ink-2)] mt-1 max-w-[380px]">
                Add a repo folder to give it its own skills. skillshare sets it up with{' '}
                <code className="font-mono text-xs">init -p</code>.
              </p>
            </div>
            <Button size="sm" variant="secondary" onClick={handleAddProject} loading={adding}>
              <Plus size={14} /> Add project folder
            </Button>
          </div>
        )}
      </SettingsSection>
    </div>
  );
}
