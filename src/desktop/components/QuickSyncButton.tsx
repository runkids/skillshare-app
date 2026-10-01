import { useEffect, useState } from 'react';
import { CheckCircle, FolderSync, Loader2, X, XCircle } from 'lucide-react';
import { tauriBridge } from '../api/tauri-bridge';

/** How long a result toast stays; errors get longer to read, as in the Web UI. */
const RESULT_MS = { ok: 4000, error: 8000 };

type SyncState =
  | { kind: 'idle' }
  | { kind: 'running' }
  | { kind: 'done'; ok: boolean; message: string };

/** Runs the tray's Quick Sync and shows its progress and result in a toast like the Web UI's. */
export default function QuickSyncButton({ disabled }: { disabled?: boolean }) {
  const [state, setState] = useState<SyncState>({ kind: 'idle' });

  useEffect(() => {
    if (state.kind !== 'done') return;
    const ms = state.ok ? RESULT_MS.ok : RESULT_MS.error;
    const timer = window.setTimeout(() => setState({ kind: 'idle' }), ms);
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
  const failed = state.kind === 'done' && !state.ok;
  const Icon = running ? Loader2 : failed ? XCircle : CheckCircle;
  return (
    <>
      <button
        type="button"
        onClick={run}
        disabled={running || disabled}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)] disabled:opacity-50"
        title="Quick Sync"
        aria-label="Quick Sync"
      >
        <FolderSync size={16} className={running ? 'motion-safe:animate-pulse' : undefined} />
      </button>
      {state.kind !== 'idle' && (
        <div className="fixed bottom-7 left-1/2 -translate-x-1/2 z-[70] w-max max-w-[min(36rem,calc(100vw-3rem))]">
          {/* Matches the Web UI's .ss-toast, which is dark in every theme. */}
          <div
            role={failed ? 'alert' : 'status'}
            className="animate-fade-in flex items-start gap-3 min-h-[42px] py-2.5 pl-3.5 pr-2 rounded-[11px] bg-[#1E1D1B] text-[#F4F2ED] text-[13px] font-medium shadow-[0_16px_40px_rgba(20,19,18,.24)]"
          >
            <Icon
              size={16}
              className={`shrink-0 mt-px ${
                running
                  ? 'text-[#A9C8FF] motion-safe:animate-spin'
                  : failed
                    ? 'text-[#FF9C8F]'
                    : 'text-[#8FD6A4]'
              }`}
            />
            <span className="flex-1 min-w-0 break-words leading-normal">
              {state.kind === 'done' ? state.message : 'Syncing…'}
            </span>
            {state.kind === 'done' && (
              <button
                type="button"
                onClick={() => setState({ kind: 'idle' })}
                aria-label="Dismiss"
                className="shrink-0 grid place-items-center w-6 h-6 -my-0.5 rounded-md opacity-60 hover:opacity-100 hover:bg-white/10"
              >
                <X size={14} />
              </button>
            )}
          </div>
        </div>
      )}
    </>
  );
}
