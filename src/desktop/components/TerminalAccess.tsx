import { useEffect, useState, type ReactNode } from 'react';
import { Check } from 'lucide-react';
import Button from '../../components/Button';
import { tauriBridge, type TerminalAccess as Access } from '../api/tauri-bridge';
import CommandBlock from './OnboardingSteps/CommandBlock';

const mono = { fontFamily: 'var(--font-mono)' };
const box =
  'col-span-2 flex flex-col gap-2 rounded-xl bg-sunken px-3.5 py-3 text-[13px] text-ink-2';
const LINK = '~/.local/bin/skillshare';

interface TerminalAccessProps {
  cliPath: string;
  /** Shown when the terminal can already run the CLI (or while checking). */
  fallback?: ReactNode;
}

/** Tells the user how to run the app's CLI from a terminal, and links the app's own copy. */
export default function TerminalAccess({ cliPath, fallback = null }: TerminalAccessProps) {
  const [access, setAccess] = useState<Access | null>(null);
  const [linking, setLinking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    // Only a hint: without it the fallback is shown.
    tauriBridge
      .cliTerminalAccess(cliPath)
      .then((a) => active && setAccess(a))
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [cliPath]);

  const link = async () => {
    setLinking(true);
    setError(null);
    try {
      setAccess(await tauriBridge.linkCliForTerminal(cliPath));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLinking(false);
    }
  };

  if (access?.needsLink) {
    return (
      <div className={box}>
        <span>
          <b className="text-ink">Use it in your terminal:</b> the CLI the app downloaded lives in
          the app’s own folder, so terminals can’t find it. Add a link at{' '}
          <span className="text-ink" style={mono}>
            {LINK}
          </span>
          .
        </span>
        <div>
          <Button size="sm" variant="secondary" loading={linking} onClick={link}>
            Use in terminal
          </Button>
        </div>
        {error && <p className="m-0 text-bad">{error}</p>}
      </div>
    );
  }

  if (access?.pathHint) {
    return (
      <div className={box}>
        <span>
          {access.linked && (
            <>
              <b className="text-ink">Linked.</b>{' '}
            </>
          )}
          <span className="text-ink" style={mono}>
            {access.pathHint.dir}
          </span>{' '}
          isn’t on your PATH yet. Run this once, then open a new terminal window.
        </span>
        <CommandBlock command={access.pathHint.command} />
      </div>
    );
  }

  if (access?.linked) {
    return (
      <div className={`${box} !flex-row items-center`}>
        <Check size={16} strokeWidth={2.5} className="flex-none text-ok" />
        <span>
          Linked to{' '}
          <span className="text-ink" style={mono}>
            {LINK}
          </span>
          . Run{' '}
          <span className="text-ink" style={mono}>
            skillshare sync
          </span>{' '}
          in any terminal.
        </span>
      </div>
    );
  }

  return fallback;
}
