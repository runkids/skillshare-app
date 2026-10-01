import { ArrowUpCircle, HeartPulse, RefreshCw, ShieldAlert, Upload } from 'lucide-react';
import type { StatusIcon, StatusRow, StatusTone } from './status-rows';

const TONES: Record<StatusTone, string> = {
  bad: 'bg-[var(--bad-bg)] text-[var(--bad)]',
  accent: 'bg-[var(--accent-bg)] text-[var(--accent)]',
  warn: 'bg-[var(--warn-bg)] text-[var(--warn)]',
  neutral: 'bg-[var(--sunken)] text-[var(--ink-2)]',
  ok: 'bg-[var(--ok-bg)] text-[var(--ok)]',
};

const ICONS: Record<StatusIcon, typeof ShieldAlert> = {
  shield: ShieldAlert,
  update: ArrowUpCircle,
  sync: RefreshCw,
  upload: Upload,
  server: HeartPulse,
};

function actionLabel(row: StatusRow) {
  switch (row.action?.kind) {
    case 'open':
      return row.action.label;
    case 'restart':
      return 'Restart';
    default:
      return null;
  }
}

interface Props {
  id: string;
  rows: StatusRow[];
  checked: string;
  checking: boolean;
  onCheckNow: () => void;
  onAction: (row: StatusRow) => void;
}

/** Everything that needs attention, one row each, with the action that resolves it. */
export default function StatusPanel({ id, rows, checked, checking, onCheckNow, onAction }: Props) {
  const primary = rows.find((row) => row.action);
  return (
    <div
      id={id}
      role="dialog"
      aria-label="Status"
      className="absolute right-0 top-full mt-2 z-50 w-[392px] rounded-[var(--r-box)] border border-[var(--line-2)] bg-[var(--surface)] shadow-[0_16px_40px_rgba(20,19,18,.14)]"
    >
      <div className="flex items-center justify-between gap-3 px-4 pt-3.5 pb-2">
        <span className="text-[14px] font-bold text-[var(--ink)]">Status</span>
        <span className="text-xs text-[var(--ink-2)]">
          {checking ? 'Checking…' : `Checked ${checked}`} ·{' '}
          <button
            type="button"
            onClick={onCheckNow}
            disabled={checking}
            className="text-[var(--accent)] hover:underline disabled:opacity-50"
          >
            Check now
          </button>
        </span>
      </div>
      <ul className="px-2 pb-2">
        {rows.map((row) => {
          const Icon = ICONS[row.icon];
          const label = actionLabel(row);
          return (
            <li
              key={row.id}
              data-testid={`status-row-${row.id}`}
              className="flex items-center gap-3 px-2 py-2"
            >
              <span
                className={`w-7 h-7 shrink-0 flex items-center justify-center rounded-[var(--r-ctl)] ${TONES[row.tone]}`}
              >
                <Icon size={15} />
              </span>
              <span className="flex-1 min-w-0">
                <span className="block text-[13px] font-semibold text-[var(--ink)]">
                  {row.title}
                </span>
                <span className="block text-xs text-[var(--ink-2)] truncate" title={row.detail}>
                  {row.detail}
                </span>
              </span>
              {label && (
                <button
                  type="button"
                  onClick={() => onAction(row)}
                  className={`shrink-0 h-[26px] px-3 rounded-[var(--r-btn)] text-xs font-medium transition-opacity hover:opacity-80 ${
                    row === primary
                      ? 'bg-[var(--ink)] text-[var(--surface)]'
                      : 'border border-[var(--line-2)] text-[var(--ink)]'
                  }`}
                >
                  {label}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
