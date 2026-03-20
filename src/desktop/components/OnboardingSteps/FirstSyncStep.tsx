import { useEffect, useState } from 'react';
import { CheckCircle, AlertCircle } from 'lucide-react';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { tauriBridge } from '../../api/tauri-bridge';

interface FirstSyncStepProps {
  cliPath: string;
  onComplete: () => void;
}

type Phase = 'syncing' | 'done' | 'error';

export default function FirstSyncStep({ cliPath, onComplete }: FirstSyncStepProps) {
  const [phase, setPhase] = useState<Phase>('syncing');
  const [output, setOutput] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function runSync() {
      try {
        const result = await tauriBridge.runCli(cliPath, ['sync']);
        if (cancelled) return;
        setOutput(result);
        setPhase('done');
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setPhase('error');
      }
    }

    runSync();
    return () => {
      cancelled = true;
    };
  }, [cliPath]);

  const handleRetry = async () => {
    setPhase('syncing');
    setError(null);
    try {
      const result = await tauriBridge.runCli(cliPath, ['sync']);
      setOutput(result);
      setPhase('done');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setPhase('error');
    }
  };

  return (
    <div className="space-y-6 text-center">
      <h2 className="text-3xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        First Sync
      </h2>
      <p className="text-pencil-light mx-auto">
        Syncing your dotfiles to build the initial snapshot.
      </p>

      <div className="min-h-[120px] flex flex-col items-center justify-center gap-4">
        {phase === 'syncing' && (
          <div className="flex items-center gap-3 text-pencil-light">
            <Spinner size="md" />
            <span>Running sync...</span>
          </div>
        )}

        {phase === 'done' && (
          <>
            <div className="flex items-center gap-2 text-success">
              <CheckCircle size={20} strokeWidth={2.5} />
              <span className="font-medium">Sync complete</span>
            </div>
            {output && (
              <pre className="text-xs text-pencil-light bg-muted/30 p-3 rounded-[var(--radius-sm)] w-full overflow-x-auto max-h-32 text-left">
                {output}
              </pre>
            )}
            <Button onClick={onComplete}>Enter skillshare App</Button>
          </>
        )}

        {phase === 'error' && (
          <>
            <div className="flex items-center gap-2 text-danger">
              <AlertCircle size={20} strokeWidth={2.5} />
              <span className="font-medium">Sync failed</span>
            </div>
            {error && <p className="text-sm text-danger ">{error}</p>}
            <div className="flex gap-3">
              <Button variant="secondary" onClick={handleRetry}>
                Retry
              </Button>
              <Button onClick={onComplete}>Skip & Continue</Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
