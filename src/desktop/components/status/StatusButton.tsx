import { useEffect, useRef, useState } from 'react';
import { ShieldAlert } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';
import { useAudit } from '../../hooks/useAudit';
import { useServerStopped, clearServerStopped } from '../../hooks/useServerStopped';
import { useSourceHealth } from '../../hooks/useSourceHealth';
import { useUpdates } from '../../hooks/useUpdates';
import { relativeTime } from '../../utils/activity';
import StatusPanel from './StatusPanel';
import { statusRows, statusSummary, type StatusRow } from './status-rows';

const PANEL_ID = 'status-panel';

/** One title bar entry for updates, source health, the audit and the server. */
export default function StatusButton() {
  const navigate = useNavigate();
  const { appInfo } = useTauri();
  const { reloadView } = useProjects();
  const findings = useAudit();
  const updates = useUpdates();
  const health = useSourceHealth();
  const stopped = useServerStopped();
  const [open, setOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checked, setChecked] = useState(() => ({ findings, updates, health, at: new Date() }));
  const root = useRef<HTMLDivElement>(null);

  // Each store publishes a new value when a check reports in.
  if (checked.findings !== findings || checked.updates !== updates || checked.health !== health) {
    setChecked({ findings, updates, health, at: new Date() });
  }

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) close();
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', keydown);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('keydown', keydown);
      window.removeEventListener('blur', close);
    };
  }, [open]);

  const rows = statusRows(findings, updates, health, {
    running: Boolean(appInfo?.serverRunning),
    port: appInfo?.serverPort ?? null,
    stopped,
  });
  const summary = statusSummary(findings, rows);
  if (!summary) return null;

  function runAction(row: StatusRow) {
    setOpen(false);
    const action = row.action;
    if (action?.kind === 'review') navigate(action.path);
    else if (action?.kind === 'restart') {
      // The header reload restarts a server that is not healthy.
      clearServerStopped();
      reloadView();
    } else if (action?.kind === 'run') {
      tauriBridge
        .runStatusAction(action.action)
        .catch((e) => console.warn(`${action.label} failed to start:`, e));
    }
  }

  function checkNow() {
    setChecking(true);
    tauriBridge
      .checkStatusNow()
      .catch((e) => console.warn('Status check failed:', e))
      .finally(() => {
        setChecking(false);
        setChecked((value) => ({ ...value, at: new Date() }));
      });
  }

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-controls={PANEL_ID}
        className={`flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] text-xs font-medium transition-opacity hover:opacity-80 ${
          summary.security
            ? 'bg-[var(--bad-bg)] text-[var(--bad)]'
            : 'bg-[var(--surface)] border border-[var(--line-2)] text-[var(--ink)]'
        }`}
      >
        {summary.security ? (
          <ShieldAlert size={13} />
        ) : (
          <span className="w-2 h-2 rounded-full bg-[var(--warn)]" />
        )}
        {summary.label}
        {summary.more > 0 && <span className="text-[var(--ink-2)]">+{summary.more}</span>}
      </button>
      {open && (
        <StatusPanel
          id={PANEL_ID}
          rows={rows}
          checked={relativeTime(checked.at, new Date())}
          checking={checking}
          onCheckNow={checkNow}
          onAction={runAction}
        />
      )}
    </div>
  );
}
