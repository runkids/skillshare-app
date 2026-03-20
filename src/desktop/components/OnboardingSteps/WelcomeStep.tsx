import { useEffect, useState } from 'react';
import { CheckCircle } from 'lucide-react';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { useCliManager } from '../../hooks/useCliManager';
import { tauriBridge } from '../../api/tauri-bridge';

interface WelcomeStepProps {
  onComplete: (cliPath: string) => void;
}

type Phase = 'checking' | 'found' | 'not-found' | 'downloading' | 'done';

export default function WelcomeStep({ onComplete }: WelcomeStepProps) {
  const { cliPath, downloading, error, detect, download } = useCliManager();
  const [phase, setPhase] = useState<Phase>('checking');
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    detect().then(async (path) => {
      if (path) {
        try {
          const v = await tauriBridge.getCliVersion(path);
          setVersion(v);
        } catch {
          // Version fetch failed — still show found state
        }
        setPhase('found');
      } else {
        setPhase('not-found');
      }
    });
  }, [detect]);

  const handleDownload = async () => {
    setPhase('downloading');
    const path = await download();
    if (path) {
      try {
        const v = await tauriBridge.getCliVersion(path);
        setVersion(v);
      } catch {
        // Version fetch failed
      }
      setPhase('done');
    } else {
      setPhase('not-found');
    }
  };

  const handleContinue = () => {
    if (cliPath) onComplete(cliPath);
  };

  // Auto-advance when done
  useEffect(() => {
    if (phase === 'done' && cliPath) {
      const timer = setTimeout(() => onComplete(cliPath), 800);
      return () => clearTimeout(timer);
    }
  }, [phase, cliPath, onComplete]);

  return (
    <div className="text-center space-y-6">
      <h2 className="text-3xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        Welcome to skillshare
      </h2>
      <p className="text-pencil-light mx-auto">
        Let&apos;s get you set up. First, we need the skillshare CLI to manage your dotfiles.
      </p>

      <div className="min-h-[120px] flex items-center justify-center">
        {phase === 'checking' && (
          <div className="flex items-center gap-3 text-pencil-light">
            <Spinner size="md" />
            <span>Checking for existing CLI...</span>
          </div>
        )}

        {phase === 'found' && (
          <div className="space-y-4">
            <div className="flex items-center justify-center gap-2 text-success">
              <CheckCircle size={20} strokeWidth={2.5} />
              <span className="font-medium">CLI found{version ? ` (${version})` : ''}</span>
            </div>
            <Button onClick={handleContinue}>Use Existing CLI</Button>
          </div>
        )}

        {phase === 'not-found' && (
          <div className="space-y-4">
            <p className="text-pencil-light text-sm">No skillshare CLI detected on this system.</p>
            {error && <p className="text-danger text-sm">{error}</p>}
            <Button onClick={handleDownload}>Download skillshare CLI</Button>
          </div>
        )}

        {phase === 'downloading' && (
          <div className="space-y-3">
            <div className="flex items-center justify-center gap-3 text-pencil-light">
              <Spinner size="md" />
              <span>Downloading CLI...</span>
            </div>
            {downloading && (
              <div className="w-48 mx-auto h-1.5 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full bg-pencil animate-shimmer rounded-full"
                  style={{ width: '60%' }}
                />
              </div>
            )}
          </div>
        )}

        {phase === 'done' && (
          <div className="flex items-center justify-center gap-2 text-success">
            <CheckCircle size={20} strokeWidth={2.5} />
            <span className="font-medium">CLI ready{version ? ` (${version})` : ''}</span>
          </div>
        )}
      </div>
    </div>
  );
}
