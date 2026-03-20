import { useState } from 'react';
import { Folder, Globe, CheckCircle } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { tauriBridge } from '../../api/tauri-bridge';

interface ProjectSetupStepProps {
  cliPath: string;
  onComplete: () => void;
}

type ProjectChoice = 'global' | 'project' | null;
type Phase = 'choose' | 'initializing' | 'done';

export default function ProjectSetupStep({ cliPath, onComplete }: ProjectSetupStepProps) {
  const [choice, setChoice] = useState<ProjectChoice>(null);
  const [phase, setPhase] = useState<Phase>('choose');
  const [selectedDir, setSelectedDir] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSelectDir = async () => {
    const dir = await open({ directory: true, title: 'Select project directory' });
    if (typeof dir === 'string') {
      setSelectedDir(dir);
    }
  };

  const handleInit = async () => {
    setPhase('initializing');
    setError(null);
    try {
      let result: string;
      if (choice === 'global') {
        const home = await homeDir();
        result = await tauriBridge.runCli(cliPath, ['init'], home);
        await tauriBridge.addProject('Global', home, 'global');
      } else if (choice === 'project' && selectedDir) {
        result = await tauriBridge.runCli(cliPath, ['init', '-p'], selectedDir);
        const name = selectedDir.split('/').pop() || 'Project';
        await tauriBridge.addProject(name, selectedDir, 'project');
      } else {
        setError('Please select a directory first.');
        setPhase('choose');
        return;
      }
      setOutput(result);
      setPhase('done');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      // "already initialized" is not a real error — treat as success
      if (msg.includes('already initialized')) {
        if (choice === 'global') {
          const home = await homeDir();
          await tauriBridge.addProject('Global', home, 'global');
        } else if (choice === 'project' && selectedDir) {
          const name = selectedDir.split('/').pop() || 'Project';
          await tauriBridge.addProject(name, selectedDir, 'project');
        }
        setOutput('Already initialized — using existing configuration.');
        setPhase('done');
        return;
      }
      setError(msg);
      setPhase('choose');
    }
  };

  // Auto-advance after success
  if (phase === 'done') {
    setTimeout(onComplete, 1200);
  }

  const optionClass = (value: ProjectChoice) =>
    `flex items-center gap-3 p-4 border-2 cursor-pointer transition-all duration-150 rounded-[var(--radius-md)] ${
      choice === value ? 'border-pencil bg-muted/30' : 'border-muted hover:border-pencil-light'
    }`;

  return (
    <div className="space-y-6 text-center">
      <div className="text-center">
        <h2
          className="text-3xl font-bold text-pencil"
          style={{ fontFamily: 'var(--font-heading)' }}
        >
          Set Up Your Project
        </h2>
        <p className="text-pencil-light mt-2 mx-auto">
          Choose how you want to manage your dotfiles.
        </p>
      </div>

      {phase === 'choose' && (
        <>
          <div className="space-y-3 mx-auto">
            <button
              type="button"
              className={optionClass('global')}
              onClick={() => {
                setChoice('global');
                setSelectedDir(null);
              }}
            >
              <Globe size={20} strokeWidth={2.5} className="text-pencil-light shrink-0" />
              <div className="text-left">
                <div className="font-medium text-pencil">Global</div>
                <div className="text-sm text-pencil-light">
                  Manage all dotfiles from your home directory
                </div>
              </div>
            </button>

            <button
              type="button"
              className={optionClass('project')}
              onClick={() => setChoice('project')}
            >
              <Folder size={20} strokeWidth={2.5} className="text-pencil-light shrink-0" />
              <div className="text-left">
                <div className="font-medium text-pencil">Project</div>
                <div className="text-sm text-pencil-light">
                  Manage dotfiles for a specific project directory
                </div>
              </div>
            </button>
          </div>

          {choice === 'project' && (
            <div className="flex items-center gap-3 mx-auto">
              <Button variant="secondary" size="sm" onClick={handleSelectDir}>
                Select Directory
              </Button>
              {selectedDir && (
                <span className="text-sm text-pencil-light truncate">{selectedDir}</span>
              )}
            </div>
          )}

          {error && <p className="text-danger text-sm text-center">{error}</p>}

          <div className="text-center">
            <Button
              onClick={handleInit}
              disabled={!choice || (choice === 'project' && !selectedDir)}
            >
              Initialize
            </Button>
          </div>
        </>
      )}

      {phase === 'initializing' && (
        <div className="flex flex-col items-center gap-3">
          <Spinner size="md" />
          <span className="text-pencil-light">Running skillshare init...</span>
        </div>
      )}

      {phase === 'done' && (
        <div className="flex flex-col items-center gap-3">
          <div className="flex items-center gap-2 text-success">
            <CheckCircle size={20} strokeWidth={2.5} />
            <span className="font-medium">Project initialized</span>
          </div>
          {output && (
            <pre className="text-xs text-pencil-light bg-muted/30 p-3 rounded-[var(--radius-sm)] w-full overflow-x-auto max-h-32">
              {output}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}
