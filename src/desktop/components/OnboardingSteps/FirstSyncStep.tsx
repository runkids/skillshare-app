import { useCallback, useEffect, useRef, useState } from 'react';
import { AlertCircle, ArrowRight, Check } from 'lucide-react';
import Button from '../../../components/Button';
import { tauriBridge, type PathHint } from '../../api/tauri-bridge';
import CommandBlock from './CommandBlock';
import StepHeader from './StepHeader';
import SyncDiagram from './SyncDiagram';
import { parseSyncResult, parseTargetNames } from './onboarding-cli';

interface FirstSyncStepProps {
  cliPath: string;
  onComplete: () => void;
  /** Fires once the sync has succeeded, before the user opens the dashboard. */
  onSynced?: () => void;
}

type Phase = 'syncing' | 'done' | 'error';

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const mono = { fontFamily: 'var(--font-mono)' };

interface SyncSink {
  isAlive: () => boolean;
  setNames: (names: string[]) => void;
  setLinked: (linked: Record<string, number> | null) => void;
  setRevealed: (count: number) => void;
  setDone: () => void;
  setFailed: (message: string) => void;
}

/** Sync for real, then reveal each target's result in turn so the diagram follows the CLI report. */
async function runFirstSync(cliPath: string, sink: SyncSink) {
  let targets: string[] = [];
  try {
    targets = parseTargetNames(await tauriBridge.runCli(cliPath, ['target', 'list', '--json']));
  } catch {
    // The diagram is decoration; names fall back to the sync report below.
  }
  if (!sink.isAlive()) return;
  sink.setNames(targets);

  try {
    // Hold the animation long enough to be seen; completion still waits for the real result.
    const [out] = await Promise.all([
      tauriBridge.runCli(cliPath, ['sync', '--json']),
      sleep(Math.min(3000, 900 + targets.length * 220)),
    ]);
    if (!sink.isAlive()) return;
    const summary = parseSyncResult(out);
    const finalNames = targets.length > 0 ? targets : Object.keys(summary?.perTarget ?? {});
    sink.setNames(finalNames);
    sink.setLinked(summary?.perTarget ?? null);
    for (let i = 1; i <= finalNames.length; i++) {
      sink.setRevealed(i);
      await sleep(180);
      if (!sink.isAlive()) return;
    }
    await sleep(finalNames.length > 0 ? 350 : 0);
    if (sink.isAlive()) sink.setDone();
  } catch (err) {
    if (sink.isAlive()) sink.setFailed(err instanceof Error ? err.message : String(err));
  }
}

export default function FirstSyncStep({ cliPath, onComplete, onSynced }: FirstSyncStepProps) {
  const [phase, setPhase] = useState<Phase>('syncing');
  const [error, setError] = useState<string | null>(null);
  const [names, setNames] = useState<string[]>([]);
  const [linked, setLinked] = useState<Record<string, number> | null>(null);
  const [revealed, setRevealed] = useState(0);
  const alive = useRef(true);
  const [pathHint, setPathHint] = useState<PathHint | null>(null);

  useEffect(() => {
    let active = true;
    // Only a hint: without it the plain terminal tip is shown.
    tauriBridge
      .cliPathHint(cliPath)
      .then((hint) => active && setPathHint(hint))
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [cliPath]);

  const execute = useCallback(
    () =>
      runFirstSync(cliPath, {
        isAlive: () => alive.current,
        setNames,
        setLinked,
        setRevealed,
        setDone: () => setPhase('done'),
        setFailed: (msg) => {
          setError(msg);
          setPhase('error');
        },
      }),
    [cliPath]
  );

  useEffect(() => {
    alive.current = true;
    void execute();
    return () => {
      alive.current = false;
    };
  }, [execute]);

  useEffect(() => {
    if (phase === 'done') onSynced?.();
  }, [phase, onSynced]);

  const handleRetry = () => {
    setPhase('syncing');
    setError(null);
    setRevealed(0);
    setLinked(null);
    void execute();
  };

  const targets = names.map((name, i) => ({
    name,
    done: i < revealed,
    linked: linked?.[name],
  }));
  const done = phase === 'done';

  return (
    <div className="ob-enter flex h-full flex-col gap-4">
      <div key={done ? 'done' : 'syncing'} className="ob-rise-item flex flex-col gap-4">
        <StepHeader
          step={3}
          title={
            done ? 'You’re all set' : phase === 'error' ? 'Sync failed' : 'Syncing your skills…'
          }
        >
          {done
            ? 'The first sync finished. From now on, add or edit a skill once and sync sends it everywhere.'
            : phase === 'error'
              ? undefined
              : 'Sending your skills to every target. This only takes a moment.'}
        </StepHeader>
      </div>

      {phase === 'error' && (
        <div className="flex items-start gap-2 text-bad">
          <AlertCircle size={20} strokeWidth={2.5} className="mt-0.5 flex-none" />
          <p className="m-0 whitespace-pre-wrap text-sm">{error}</p>
        </div>
      )}

      {phase !== 'error' && targets.length > 0 && (
        <div className="h-[210px] min-h-0">
          <SyncDiagram targets={targets} />
        </div>
      )}
      {done && targets.length === 0 && (
        <div className="flex items-center gap-2 text-ok">
          <Check size={20} strokeWidth={2.5} />
          <span className="font-medium">
            Sync complete. No targets are configured yet — add one from the Targets page.
          </span>
        </div>
      )}

      {done && (
        <div className="ob-enter grid grid-cols-2 gap-2.5" aria-live="polite">
          <div className="rounded-xl bg-sunken px-3.5 py-3 text-[13px] text-ink-2">
            <b className="text-ink">Next:</b> install a skill from GitHub on the Skills page.
          </div>
          {!pathHint && (
            <div className="rounded-xl bg-sunken px-3.5 py-3 text-[13px] text-ink-2">
              <b className="text-ink">Tip:</b> the CLI works in your terminal too —{' '}
              <span className="text-ink" style={mono}>
                skillshare sync
              </span>
            </div>
          )}
          {pathHint && (
            <div className="col-span-2 flex flex-col gap-2 rounded-xl bg-sunken px-3.5 py-3 text-[13px] text-ink-2">
              <span>
                <b className="text-ink">Use it in your terminal:</b>{' '}
                <span className="text-ink" style={mono}>
                  {pathHint.dir}
                </span>{' '}
                isn’t on your PATH yet. Run this once, then open a new terminal window.
              </span>
              <CommandBlock command={pathHint.command} />
            </div>
          )}
        </div>
      )}

      <div className="mt-auto flex items-center gap-3">
        {done && (
          <div className="ob-enter">
            <Button size="lg" autoFocus onClick={onComplete}>
              Open dashboard <ArrowRight size={16} />
            </Button>
          </div>
        )}
        {phase === 'error' && (
          <>
            <Button size="lg" onClick={handleRetry}>
              Retry
            </Button>
            <Button size="lg" variant="secondary" onClick={onComplete}>
              Skip &amp; Continue
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
