import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  AlertCircle,
  ArrowLeft,
  ArrowUpCircle,
  Download,
  FolderInput,
  History,
  RefreshCw,
  RotateCw,
  ShieldCheck,
  Trash2,
} from 'lucide-react';
import { tauriBridge, SYNC_COMPLETED_EVENT, type ActivityEntry } from '../api/tauri-bridge';
import { useProjects } from '../context/ProjectContext';
import { groupByDay, relativeTime } from '../utils/activity';
import { isMacOS } from '../utils/platform';

const ICONS: Record<string, typeof History> = {
  sync: RefreshCw,
  install: Download,
  update: ArrowUpCircle,
  collect: FolderInput,
  uninstall: Trash2,
  audit: ShieldCheck,
};

function ActivityRow({ entry, now }: { entry: ActivityEntry; now: Date }) {
  const failed = entry.status === 'error' || entry.status === 'blocked';
  const partial = entry.status === 'partial';
  const Icon = failed ? AlertCircle : (ICONS[entry.cmd] ?? History);
  const date = new Date(entry.ts);
  return (
    <li
      data-status={entry.status}
      className={`flex gap-3 px-4 py-3 rounded-[var(--r-box)] border-[length:var(--bw)] ${
        failed
          ? 'bg-[var(--bad-bg)] border-[var(--bad)]'
          : 'bg-[var(--surface)] border-[var(--line)]'
      }`}
    >
      <Icon
        size={16}
        className={`mt-0.5 shrink-0 ${
          failed ? 'text-[var(--bad)]' : partial ? 'text-[var(--warn)]' : 'text-[var(--ink-2)]'
        }`}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-semibold text-[var(--ink)] capitalize">{entry.cmd}</span>
          {entry.status !== 'ok' && (
            <span
              className={`text-[11px] font-bold px-[7px] py-0.5 rounded-[var(--r-btn)] ${
                failed
                  ? 'bg-[var(--bad)] text-[var(--surface)]'
                  : 'bg-[var(--warn-bg)] text-[var(--warn)]'
              }`}
            >
              {entry.status}
            </span>
          )}
          {entry.detail && <span className="text-xs text-[var(--ink-2)]">{entry.detail}</span>}
          <time
            dateTime={entry.ts}
            title={date.toLocaleString()}
            className="ml-auto shrink-0 text-xs text-[var(--ink-3)]"
          >
            {relativeTime(date, now)}
          </time>
        </div>
        {entry.subjects.length > 0 && (
          <div className="mt-1 text-[13px] text-[var(--ink-2)] break-words">
            {entry.subjects.join(', ')}
          </div>
        )}
        {entry.message && (
          <div
            className={`mt-1 text-[13px] break-words ${
              failed ? 'text-[var(--bad)]' : 'text-[var(--ink-2)]'
            }`}
          >
            {entry.message}
          </div>
        )}
      </div>
    </li>
  );
}

export default function ActivityPage() {
  const navigate = useNavigate();
  const { activeProject } = useProjects();
  const [entries, setEntries] = useState<ActivityEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [now, setNow] = useState(() => new Date());

  const load = useCallback(
    () =>
      tauriBridge
        .getActivity()
        .then((list) => {
          setEntries(list);
          setError(null);
        })
        .catch((err) => setError(err instanceof Error ? err.message : String(err)))
        .finally(() => setNow(new Date())),
    []
  );

  // The log is per project; re-read it when the active one changes.
  useEffect(() => {
    if (!isTauri()) return;
    void load();
  }, [load, activeProject?.id]);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen(SYNC_COMPLETED_EVENT, () => void load());
    return () => {
      void unlisten.then((off) => off());
    };
  }, [load]);

  const refresh = () => {
    setRefreshing(true);
    void load().finally(() => setRefreshing(false));
  };

  const renderBody = () => {
    if (!isTauri()) {
      return <p className="text-sm text-[var(--ink-2)]">Activity is available in the app.</p>;
    }
    if (error) {
      return (
        <p role="alert" className="text-sm text-[var(--bad)]">
          Could not read the activity log: {error}
        </p>
      );
    }
    if (!entries) return <p className="text-sm text-[var(--ink-2)]">Loading…</p>;
    if (entries.length === 0) {
      return <p className="text-sm text-[var(--ink-2)]">No activity in the last 7 days.</p>;
    }
    return groupByDay(entries, now).map((day) => (
      <section key={day.label} aria-label={day.label} className="flex flex-col gap-2">
        <h2 className="text-[15px] font-bold text-[var(--ink)]">{day.label}</h2>
        <ul className="flex flex-col gap-2">
          {day.entries.map((entry, i) => (
            <ActivityRow key={`${entry.ts}-${entry.cmd}-${i}`} entry={entry} now={now} />
          ))}
        </ul>
      </section>
    ));
  };

  return (
    <main
      className="h-screen overflow-y-auto bg-[var(--bg)] px-16 pb-12"
      // Leave room for the macOS overlay traffic lights; this page has no title bar.
      style={{ paddingTop: isMacOS() ? '48px' : '20px' }}
    >
      <div className="max-w-[720px] flex flex-col gap-6">
        <button
          type="button"
          onClick={() => navigate('/')}
          className="self-start flex items-center gap-2 h-9 px-2.5 -ml-2.5 rounded-[var(--r-ctl)] text-sm text-[var(--ink-2)] hover:text-[var(--ink)] hover:bg-[var(--sel)] transition-colors"
        >
          <ArrowLeft size={18} />
          <span>Back to dashboard</span>
        </button>
        <div className="flex items-start gap-4">
          <div className="flex-1">
            <h1 className="text-2xl font-bold text-[var(--ink)]">Activity</h1>
            <p className="text-sm text-[var(--ink-2)] mt-1">
              Syncs, installs, updates and errors from the last 7 days
              {activeProject ? ` in ${activeProject.name}` : ''}.
            </p>
          </div>
          <button
            type="button"
            onClick={refresh}
            disabled={refreshing || !isTauri()}
            className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)] disabled:opacity-50"
            title="Refresh"
            aria-label="Refresh"
          >
            <RotateCw size={15} className={refreshing ? 'motion-safe:animate-spin' : undefined} />
          </button>
        </div>
        {renderBody()}
      </div>
    </main>
  );
}
