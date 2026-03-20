import { useEffect, useState } from 'react';
import { CheckCircle } from 'lucide-react';
import { homeDir } from '@tauri-apps/api/path';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { tauriBridge } from '../../api/tauri-bridge';

interface ProjectSetupStepProps {
  cliPath: string;
  onComplete: () => void;
}

type Phase = 'initializing' | 'done' | 'error';

export default function ProjectSetupStep({ cliPath, onComplete }: ProjectSetupStepProps) {
  const [phase, setPhase] = useState<Phase>('initializing');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function initGlobal() {
      try {
        const home = await homeDir();
        try {
          await tauriBridge.runCli(cliPath, ['init'], home);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          if (!msg.includes('already initialized')) throw err;
        }
        await tauriBridge.addProject('Global', home, 'global');
        if (!cancelled) setPhase('done');
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
        setPhase('error');
      }
    }

    initGlobal();
    return () => { cancelled = true; };
  }, [cliPath]);

  // Auto-advance after success
  useEffect(() => {
    if (phase === 'done') {
      const timer = setTimeout(onComplete, 1200);
      return () => clearTimeout(timer);
    }
  }, [phase, onComplete]);

  const handleRetry = async () => {
    setPhase('initializing');
    setError(null);
    try {
      const home = await homeDir();
      try {
        await tauriBridge.runCli(cliPath, ['init'], home);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (!msg.includes('already initialized')) throw err;
      }
      await tauriBridge.addProject('Global', home, 'global');
      setPhase('done');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase('error');
    }
  };

  return (
    <div className="space-y-6 text-center">
      <div className="text-center">
        <h2
          className="text-3xl font-bold text-pencil"
          style={{ fontFamily: 'var(--font-heading)' }}
        >
          Setting Up Global Config
        </h2>
        <p className="text-pencil-light mt-2 mx-auto">
          Initializing skillshare in your home directory.
        </p>
      </div>

      <div className="min-h-[120px] flex flex-col items-center justify-center gap-4">
        {phase === 'initializing' && (
          <div className="flex items-center gap-3 text-pencil-light">
            <Spinner size="md" />
            <span>Running skillshare init...</span>
          </div>
        )}

        {phase === 'done' && (
          <div className="flex items-center gap-2 text-success">
            <CheckCircle size={20} strokeWidth={2.5} />
            <span className="font-medium">Global config initialized</span>
          </div>
        )}

        {phase === 'error' && (
          <>
            <p className="text-danger text-sm">{error}</p>
            <Button onClick={handleRetry}>Retry</Button>
          </>
        )}
      </div>
    </div>
  );
}
