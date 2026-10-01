import { useEffect, useState } from 'react';
import { FolderSync } from 'lucide-react';
import { tauriBridge } from '../api/tauri-bridge';

/** How long a result stays below the button. */
const RESULT_MS = 6000;

type SyncState =
  | { kind: 'idle' }
  | { kind: 'running' }
  | { kind: 'done'; ok: boolean; message: string };

/** Runs the tray's Quick Sync and shows its progress and result in a card below the button. */
export default function QuickSyncButton({ disabled }: { disabled?: boolean }) {
  const [state, setState] = useState<SyncState>({ kind: 'idle' });

  useEffect(() => {
    if (state.kind !== 'done') return;
    const timer = window.setTimeout(() => setState({ kind: 'idle' }), RESULT_MS);
    return () => window.clearTimeout(timer);
  }, [state]);

  const run = () => {
    setState({ kind: 'running' });
    tauriBridge
      .quickSync()
      .then((message) => setState({ kind: 'done', ok: true, message }))
      .catch((error: unknown) => setState({ kind: 'done', ok: false, message: String(error) }));
  };

  const running = state.kind === 'running';
  const done = state.kind === 'done';
  return (
    <div className="relative">
      <button
        type="button"
        onClick={run}
        disabled={running || disabled}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)] disabled:opacity-50"
        // The native tooltip would cover the card.
        title={running || done ? undefined : 'Quick Sync'}
        aria-label="Quick Sync"
      >
        <FolderSync size={16} className={running ? 'motion-safe:animate-pulse' : undefined} />
      </button>
      {(running || done) && (
        <div
          role={done && !state.ok ? 'alert' : 'status'}
          onClick={done ? () => setState({ kind: 'idle' }) : undefined}
          title={done ? 'Click to dismiss' : undefined}
          className={`absolute right-0 top-full mt-2 z-50 w-max max-w-[320px] px-3 py-2 rounded-[var(--r-ctl)] border border-[var(--line-2)] bg-[var(--surface)] shadow-[0_16px_40px_rgba(20,19,18,.14)] text-xs ${
            done && !state.ok ? 'text-danger' : 'text-[var(--ink)]'
          } ${done ? 'cursor-pointer' : ''}`}
        >
          {done ? state.message : 'Syncing…'}
        </div>
      )}
    </div>
  );
}
