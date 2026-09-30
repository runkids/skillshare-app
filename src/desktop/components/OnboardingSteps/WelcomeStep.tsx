import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowRight, Check, Monitor } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { useCliManager } from '../../hooks/useCliManager';
import { tauriBridge } from '../../api/tauri-bridge';
import CommandBlock from './CommandBlock';
import StepHeader from './StepHeader';
import StepMark from './StepMark';
import { getInstallMethods, platformLabel, type InstallMethodId } from './install-methods';

interface WelcomeStepProps {
  onComplete: (cliPath: string) => void;
}

type Phase = 'checking' | 'found' | 'missing' | 'manual' | 'installing';

const mono = { fontFamily: 'var(--font-mono)' };

export default function WelcomeStep({ onComplete }: WelcomeStepProps) {
  const cli = useCliManager();
  const { detect, loadPlatform, install, cancelInstall, resetInstall } = cli;
  const [phase, setPhase] = useState<Phase>('checking');
  const [version, setVersion] = useState<string | null>(null);
  const [foundPath, setFoundPath] = useState<string | null>(null);
  const [methodId, setMethodId] = useState<InstallMethodId | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const logRef = useRef<HTMLPreElement>(null);

  const plan = cli.platform ? getInstallMethods(cli.platform) : null;
  const method = plan?.methods.find((m) => m.id === (methodId ?? plan.recommended)) ?? null;

  const showMissing = useCallback(async () => {
    const platform = await loadPlatform();
    if (platform) setMethodId(getInstallMethods(platform).recommended);
    setPhase('missing');
  }, [loadPlatform]);

  useEffect(() => {
    let cancelled = false;
    detect().then(async (path) => {
      if (cancelled) return;
      if (path) {
        try {
          setVersion(await tauriBridge.getCliVersion(path));
        } catch (err) {
          setNotice(
            `Found the CLI but could not read its version: ${err instanceof Error ? err.message : err}`
          );
        }
        if (cancelled) return;
        setFoundPath(path);
        setPhase('found');
      } else {
        await showMissing();
      }
    });
    return () => {
      cancelled = true;
    };
  }, [detect, showMissing]);

  // Auto-advance shortly after a verified install; Continue stays available meanwhile.
  useEffect(() => {
    if (phase === 'installing' && cli.installPhase === 'verified' && cli.cliPath) {
      const path = cli.cliPath;
      const timer = setTimeout(() => onComplete(path), 1500);
      return () => clearTimeout(timer);
    }
  }, [phase, cli.installPhase, cli.cliPath, onComplete]);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [cli.lines.length]);

  const handleInstall = async () => {
    if (!method) return;
    setNotice(null);
    setPhase('installing');
    const path = await install(method.id, method.command);
    if (!path) setPhase('missing');
  };

  const handleCancel = async () => {
    await cancelInstall();
    setPhase('missing');
  };

  const handleRecheck = async () => {
    setNotice(null);
    const path = await detect();
    if (path) {
      try {
        setVersion(await tauriBridge.getCliVersion(path));
      } catch (err) {
        setNotice(
          `Found the CLI but could not read its version: ${err instanceof Error ? err.message : err}`
        );
      }
      setFoundPath(path);
      setPhase('found');
    } else {
      setNotice('skillshare is still not on this machine. Run the command, then check again.');
    }
  };

  const handleLocate = async () => {
    setNotice(null);
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        title: 'Select the skillshare binary',
      });
      if (typeof picked !== 'string') return;
      const v = await tauriBridge.getCliVersion(picked);
      setVersion(v);
      setFoundPath(picked);
      setPhase('found');
    } catch (err) {
      setNotice(
        `That file did not work as the skillshare CLI: ${err instanceof Error ? err.message : err}`
      );
    }
  };

  const installing = cli.installPhase === 'installing';
  const verified = cli.installPhase === 'verified';
  const progress = verified ? 1 : Math.min(0.9, 0.15 + (cli.lines.length - 1) * 0.05);

  if (phase === 'checking') {
    return (
      <div className="ob-enter flex flex-col gap-4">
        <StepHeader step={1} title="Looking for the CLI">
          Checking your PATH first so you don&apos;t have to install twice.
        </StepHeader>
        <div className="flex items-center gap-3 text-ink-2" role="status">
          <Spinner size="md" />
          <span>Checking for an existing skillshare CLI…</span>
        </div>
      </div>
    );
  }

  if (phase === 'found') {
    return (
      <div className="ob-enter flex h-full flex-col gap-4">
        <StepHeader step={1} title="Let’s find your skillshare CLI">
          The app is a window onto the CLI. We looked on your PATH first so you don&apos;t have to
          install twice.
        </StepHeader>
        <div className="flex items-center gap-3 rounded-xl bg-ok-bg px-3.5 py-3">
          <span className="flex h-7 w-7 flex-none items-center justify-center rounded-full bg-ok text-on-pri">
            <Check size={15} strokeWidth={2.5} />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-sm font-bold text-ink">CLI found</div>
            <div className="truncate text-xs text-ink-2" style={mono}>
              skillshare {version ?? ''} · {foundPath}
            </div>
          </div>
          <button
            type="button"
            onClick={handleLocate}
            className="cursor-pointer border-none bg-transparent text-[13px] text-link hover:underline"
          >
            Use a different binary
          </button>
        </div>
        {notice && <p className="m-0 text-sm text-bad">{notice}</p>}
        <div className="rounded-xl border border-line-soft p-3.5 text-[13px] text-ink-2">
          <div className="mb-2 font-bold text-ink">What happens next</div>
          <div className="mb-1.5">
            <span className="mr-2 rounded bg-sunken px-1.5 py-0.5 text-ink" style={mono}>
              init
            </span>
            creates your global config in your home folder
          </div>
          <div>
            <span className="mr-2 rounded bg-sunken px-1.5 py-0.5 text-ink" style={mono}>
              sync
            </span>
            builds the first snapshot of your skills
          </div>
        </div>
        <div className="mt-auto flex items-center gap-3">
          <Button
            size="lg"
            autoFocus
            disabled={!foundPath}
            onClick={() => foundPath && onComplete(foundPath)}
          >
            Continue <ArrowRight size={16} />
          </Button>
          <span className="text-xs text-ink-3">Press Enter ↵</span>
        </div>
      </div>
    );
  }

  if (phase === 'installing') {
    const command = method?.command ?? null;
    return (
      <div className="ob-enter flex h-full min-h-0 flex-col gap-4">
        <StepHeader
          step={1}
          title={verified ? 'skillshare is installed' : 'Installing skillshare…'}
        >
          {verified
            ? `Verified skillshare ${cli.installVersion ?? ''}. Moving on to setup.`
            : method?.id === 'app-copy'
              ? 'Downloading the latest release from GitHub. This usually takes a few seconds.'
              : `Running the ${method?.label ?? ''} installer. This usually takes under a minute.`}
        </StepHeader>
        <ul
          className="m-0 flex list-none flex-col gap-3 p-0 text-[15px]"
          aria-label="Install progress"
        >
          <li className="flex items-center gap-3">
            <StepMark state="done" />
            <span className="flex-1">Checked system</span>
            {cli.platform && (
              <span className="text-xs text-ink-3">{platformLabel(cli.platform)}</span>
            )}
          </li>
          <li className={`flex items-center gap-3 ${installing ? 'font-bold' : ''}`}>
            <StepMark state={verified ? 'done' : 'active'} />
            <span className="flex-1">Downloading and installing</span>
            <span className="text-xs font-normal text-ink-3">{method?.id}</span>
          </li>
          <li className={`flex items-center gap-3 ${verified ? '' : 'text-ink-3'}`}>
            <StepMark state={verified ? 'done' : 'pending'} />
            <span>
              Verifying <span style={mono}>skillshare version</span>
            </span>
          </li>
        </ul>
        <div
          className="ob-progress"
          role="progressbar"
          aria-label="Install progress"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={cli.lines.length > 1 || verified ? Math.round(progress * 100) : undefined}
        >
          <div
            className="ob-progress-bar"
            data-mode={cli.lines.length > 1 || verified ? 'determinate' : 'indeterminate'}
            style={{ ['--ob-p' as string]: progress }}
          />
        </div>
        <details open className="min-h-0 flex-1 rounded-xl bg-ink px-3.5 py-3 text-bg">
          <summary className="cursor-pointer text-xs font-semibold">Installer output</summary>
          <pre
            ref={logRef}
            role="log"
            aria-live="off"
            className="m-0 mt-2.5 max-h-[110px] overflow-auto whitespace-pre-wrap text-xs leading-[1.6]"
            style={mono}
          >
            {cli.lines.map((l) => (
              <span
                key={l.id}
                className={`ob-line block ${l.stream === 'stderr' ? 'text-warn' : ''}`}
              >
                {l.text}
              </span>
            ))}
            {cli.lines.length === 0 && command}
          </pre>
        </details>
        <div className="flex items-center gap-3">
          <Button
            size="lg"
            disabled={!verified || !cli.cliPath}
            onClick={() => cli.cliPath && onComplete(cli.cliPath)}
          >
            Continue <ArrowRight size={16} />
          </Button>
          <Button size="lg" variant="secondary" onClick={handleCancel} disabled={verified}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  // missing | manual
  const manual = phase === 'manual';
  return (
    <div className="ob-enter flex h-full flex-col gap-4">
      <StepHeader step={1} title={manual ? 'Install it yourself' : 'We’ll install the CLI for you'}>
        {manual
          ? 'Run the command below in a terminal, then check again.'
          : 'No skillshare CLI on this machine yet. The app runs on top of it, so we’ll set it up with the installer that fits your system.'}
      </StepHeader>
      {cli.platform && (
        <div className="flex items-center gap-2.5 text-[13px] text-ink-2">
          <span className="flex items-center gap-1.5 rounded-full bg-sunken px-2.5 py-1">
            <Monitor size={14} />
            {platformLabel(cli.platform)}
          </span>
          {cli.platform.os === 'macos' && cli.platform.brew && (
            <span className="flex items-center gap-1.5 rounded-full bg-ok-bg px-2.5 py-1 text-ok">
              <Check size={14} />
              Homebrew found
            </span>
          )}
        </div>
      )}
      {plan && method && (
        <div className="overflow-hidden rounded-xl border border-line-soft">
          <div
            role="tablist"
            aria-label="Install method"
            className="flex gap-1 border-b border-line-soft bg-sunken p-1.5"
          >
            {plan.methods.map((m) => (
              <button
                key={m.id}
                type="button"
                role="tab"
                aria-selected={m.id === method.id}
                onClick={() => setMethodId(m.id)}
                className={`h-8 cursor-pointer rounded-full px-3.5 text-[13px] ${
                  m.id === method.id
                    ? 'border border-line-2 bg-surface font-bold text-ink'
                    : 'border-0 bg-transparent font-semibold text-ink-2'
                }`}
              >
                {m.label}
                {m.id === plan.recommended && ' · recommended'}
              </button>
            ))}
          </div>
          <div role="tabpanel" className="flex flex-col gap-2.5 px-[18px] py-4">
            {method.command ? (
              <CommandBlock command={method.command} />
            ) : (
              <div className="rounded-lg bg-sunken px-3.5 py-3 text-[13px] text-ink" style={mono}>
                Download the latest release from github.com/runkids/skillshare
              </div>
            )}
            <div className="text-[13px] text-ink-2">{method.note}</div>
          </div>
        </div>
      )}
      {notice && <p className="m-0 text-sm text-bad">{notice}</p>}
      {cli.error && <p className="m-0 text-sm text-bad">{cli.error}</p>}
      <div className="mt-auto flex items-center gap-3">
        {manual ? (
          <>
            <Button size="lg" onClick={handleRecheck}>
              Re-check
            </Button>
            <Button
              size="lg"
              variant="secondary"
              onClick={() => {
                setNotice(null);
                setPhase('missing');
              }}
            >
              Back
            </Button>
          </>
        ) : (
          <>
            <Button size="lg" disabled={!method} onClick={handleInstall}>
              Install skillshare CLI <ArrowRight size={16} />
            </Button>
            <Button
              size="lg"
              variant="secondary"
              onClick={() => {
                resetInstall();
                setPhase('manual');
              }}
            >
              I’ll install it myself
            </Button>
            <button
              type="button"
              onClick={handleLocate}
              className="ml-auto cursor-pointer border-none bg-transparent text-[13px] text-link hover:underline"
            >
              Locate an existing binary…
            </button>
          </>
        )}
      </div>
    </div>
  );
}
