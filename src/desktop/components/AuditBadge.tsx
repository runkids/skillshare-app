import { useEffect, useRef, useState } from 'react';
import { ShieldAlert } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useAudit } from '../hooks/useAudit';
import type { AuditFinding } from '../api/tauri-bridge';

function plural(n: number, word: string) {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

function isAlert(finding: AuditFinding) {
  return finding.severity === 'CRITICAL' || finding.severity === 'HIGH';
}

/** The CLI Web UI page for the skill or agent a finding is in. */
function resourcePath(finding: AuditFinding) {
  return `/${finding.kind === 'agent' ? 'agents' : 'skills'}/${encodeURIComponent(finding.skill)}`;
}

/** Findings from the audit after the last change; HIGH and CRITICAL stand out. */
export default function AuditBadge() {
  const findings = useAudit();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

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

  if (findings.length === 0) return null;
  const alerts = findings.filter(isAlert).length;
  // LOW findings are usually numerous and informational; count them, list the rest.
  const listed = findings.filter((f) => f.severity !== 'LOW');
  const low = findings.length - listed.length;
  const label = alerts > 0 ? `${alerts} high risk` : plural(findings.length, 'finding');

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-controls="audit-findings"
        className={`flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] text-xs transition-colors hover:text-[var(--ink)] ${
          alerts > 0
            ? 'bg-[var(--bad-bg)] text-[var(--bad)]'
            : 'text-[var(--ink-2)] hover:bg-[var(--sel)]'
        }`}
        title={`Security audit: ${plural(findings.length, 'finding')}`}
      >
        <ShieldAlert size={13} />
        {label}
      </button>
      {open && (
        <div
          id="audit-findings"
          className="absolute right-0 top-full mt-2 z-50 w-80 max-h-96 overflow-y-auto p-1 rounded-[var(--r-box)] border border-[var(--line)] bg-[var(--surface)] shadow-lg"
        >
          {listed.map((finding, i) => (
            <Link
              key={`${finding.kind}:${finding.skill}:${finding.file}:${finding.line}:${i}`}
              to={resourcePath(finding)}
              onClick={() => setOpen(false)}
              className="block px-3 py-2 rounded-[var(--r-btn)] hover:bg-[var(--sel)] text-xs text-[var(--ink)]"
            >
              <span
                className={`font-semibold ${isAlert(finding) ? 'text-[var(--bad)]' : 'text-[var(--warn)]'}`}
              >
                {finding.severity}
              </span>{' '}
              <span className="font-medium">{finding.skill}</span>
              <span className="block mt-1 text-[var(--ink-2)] break-words">
                {finding.message}
                {finding.file && ` — ${finding.file}${finding.line > 0 ? `:${finding.line}` : ''}`}
              </span>
            </Link>
          ))}
          <Link
            to="/audit"
            onClick={() => setOpen(false)}
            className="block px-3 py-2 rounded-[var(--r-btn)] hover:bg-[var(--sel)] text-xs text-[var(--accent)]"
          >
            {low > 0 ? `${plural(low, 'low finding')} · ` : ''}Open the audit page
          </Link>
        </div>
      )}
    </div>
  );
}
