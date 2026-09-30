import { useCallback, useEffect, useState } from 'react';
import { ArrowRight, Check } from 'lucide-react';
import { homeDir } from '@tauri-apps/api/path';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { tauriBridge } from '../../api/tauri-bridge';
import AnimatedCheck from './AnimatedCheck';
import StepHeader from './StepHeader';
import { DETECT_TARGETS_ARGS, parseDetectedTargets, type KnownTarget } from './onboarding-cli';

interface ProjectSetupStepProps {
  cliPath: string;
  onComplete: () => void;
}

type Phase = 'initializing' | 'choose' | 'adding' | 'error';

const mono = { fontFamily: 'var(--font-mono)' };

export default function ProjectSetupStep({ cliPath, onComplete }: ProjectSetupStepProps) {
  const [phase, setPhase] = useState<Phase>('initializing');
  const [error, setError] = useState<string | null>(null);
  const [configDir, setConfigDir] = useState<string | null>(null);
  const [detected, setDetected] = useState<KnownTarget[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const setup = useCallback(async () => {
    const home = await homeDir();
    // Non-interactive init (ignore "already initialized"); targets are chosen below.
    try {
      await tauriBridge.runCli(
        cliPath,
        ['init', '--no-copy', '--no-git', '--no-skill', '--no-targets'],
        home
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (!msg.includes('already initialized')) throw err;
    }
    // Get actual config dir from CLI status
    const dir = await tauriBridge.getGlobalConfigDir(cliPath);
    // Add to store (ignore "already exists")
    try {
      await tauriBridge.addProject('Global', dir || home, 'global');
    } catch {
      // Global already in store — that's fine
    }
    const found = parseDetectedTargets(
      await tauriBridge.runCli(cliPath, DETECT_TARGETS_ARGS, home)
    );
    return { dir: dir || home, found };
  }, [cliPath]);

  const run = useCallback(() => {
    let cancelled = false;
    setup().then(
      ({ dir, found }) => {
        if (cancelled) return;
        setConfigDir(dir);
        setDetected(found);
        setSelected(new Set(found.map((t) => t.name)));
        setPhase('choose');
      },
      (err) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
        setPhase('error');
      }
    );
    return () => {
      cancelled = true;
    };
  }, [setup]);

  useEffect(() => run(), [run]);

  const toggle = (name: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (!next.delete(name)) next.add(name);
      return next;
    });

  const handleRetry = () => {
    setPhase('initializing');
    setError(null);
    run();
  };

  const handleAdd = async () => {
    setPhase('adding');
    setError(null);
    try {
      const home = await homeDir();
      await tauriBridge.runCli(
        cliPath,
        ['init', '--discover', '--select', [...selected].join(',')],
        home
      );
      onComplete();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase('error');
    }
  };

  if (phase === 'initializing') {
    return (
      <div className="ob-enter flex flex-col gap-4">
        <StepHeader step={2} title="Setting up your config">
          Creating the global config and looking for AI tools on this machine.
        </StepHeader>
        <div className="flex items-center gap-3 text-ink-2" role="status">
          <Spinner size="md" />
          <span>Running skillshare init…</span>
        </div>
      </div>
    );
  }

  if (phase === 'error') {
    return (
      <div className="ob-enter flex h-full flex-col gap-4">
        <StepHeader step={2} title="Setup hit a problem" />
        <p className="m-0 whitespace-pre-wrap text-sm text-bad">{error}</p>
        <div className="mt-auto flex items-center gap-3">
          <Button size="lg" onClick={configDir ? handleAdd : handleRetry}>
            Retry
          </Button>
          <Button size="lg" variant="secondary" onClick={onComplete}>
            Skip for now
          </Button>
        </div>
      </div>
    );
  }

  const count = selected.size;
  return (
    <div className="ob-enter flex h-full flex-col gap-4">
      <StepHeader step={2} title="Where should your skills go?" />
      <div className="flex items-center gap-3 rounded-xl bg-ok-bg px-3.5 py-3">
        <span className="flex h-7 w-7 flex-none items-center justify-center rounded-full bg-ok text-on-pri">
          <Check size={15} strokeWidth={2.5} />
        </span>
        <span className="min-w-0 flex-1 truncate text-sm text-ok">
          <b>Global config created</b> ·{' '}
          <span className="text-xs" style={mono}>
            {configDir}
          </span>
        </span>
      </div>
      {detected.length === 0 ? (
        <p className="m-0 max-w-[520px] text-[15px] leading-[1.55] text-ink-2">
          No new AI tools were found on this machine. You can add targets later from the Targets
          page.
        </p>
      ) : (
        <>
          <p className="m-0 max-w-[520px] text-[15px] leading-[1.55] text-ink-2">
            We found these AI tools on your machine. Each one becomes a target: every sync copies
            your skills into it.
          </p>
          <fieldset className="m-0 grid min-h-0 grid-cols-2 gap-2.5 overflow-y-auto border-0 p-0">
            <legend className="sr-only">Targets</legend>
            {detected.map((t, i) => {
              const checked = selected.has(t.name);
              return (
                <label
                  key={t.name}
                  className={`ob-rise-item flex cursor-pointer items-center gap-3 rounded-xl border bg-surface px-3 py-2.5 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-link ${
                    checked ? 'border-line-2' : 'border-line-soft'
                  }`}
                  style={{ ['--ob-i' as string]: i }}
                >
                  <input
                    type="checkbox"
                    className="sr-only"
                    checked={checked}
                    onChange={() => toggle(t.name)}
                    aria-label={t.label}
                  />
                  <span
                    className={`ob-box flex h-[22px] w-[22px] flex-none items-center justify-center rounded-md border-2 ${
                      checked ? 'border-link bg-link text-on-pri' : 'border-line-2 bg-surface'
                    }`}
                    data-checked={String(checked)}
                    aria-hidden="true"
                  >
                    <AnimatedCheck size={15} />
                  </span>
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="text-sm font-bold text-ink">{t.label}</span>
                    <span className="truncate text-[11px] text-ink-3" style={mono}>
                      {t.path}
                    </span>
                  </span>
                  <span className="text-xs text-ok">Detected</span>
                </label>
              );
            })}
          </fieldset>
        </>
      )}
      <div className="mt-auto flex items-center gap-3">
        {detected.length === 0 ? (
          <Button size="lg" onClick={onComplete}>
            Continue <ArrowRight size={16} />
          </Button>
        ) : (
          <>
            <Button
              size="lg"
              loading={phase === 'adding'}
              disabled={count === 0}
              onClick={handleAdd}
            >
              Sync to {count} {count === 1 ? 'target' : 'targets'} <ArrowRight size={16} />
            </Button>
            <Button
              size="lg"
              variant="secondary"
              onClick={onComplete}
              disabled={phase === 'adding'}
            >
              Skip for now
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
