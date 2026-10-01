import { HeartPulse } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useSourceHealth } from '../hooks/useSourceHealth';
import type { SourceHealth } from '../api/tauri-bridge';

function plural(n: number, word: string) {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

/** Short labels and their tooltip lines for each finding; empty when the source is healthy. */
function describeSourceHealth(health: SourceHealth) {
  const parts: { label: string; detail: string }[] = [];
  const local = health.localSkills.length;
  if (local > 0) {
    parts.push({
      label: `${local} local`,
      detail: `${plural(local, 'skill')} only in targets: ${health.localSkills.join(', ')}`,
    });
  }
  const drift = health.outOfSyncTargets.length;
  if (drift > 0) {
    parts.push({
      label: `${drift} out of sync`,
      detail: `${plural(drift, 'target')} out of sync: ${health.outOfSyncTargets.join(', ')}`,
    });
  }
  const git = health.git;
  if (git && git.uncommitted + git.ahead > 0) {
    parts.push({
      label: `↑${git.uncommitted + git.ahead}`,
      detail: `${plural(git.uncommitted, 'uncommitted change')}, ${plural(git.ahead, 'commit')} to push`,
    });
  }
  if (git && git.behind > 0) {
    parts.push({ label: `↓${git.behind}`, detail: `${plural(git.behind, 'commit')} to pull` });
  }
  return parts;
}

export default function SourceHealthBadge() {
  const navigate = useNavigate();
  const health = useSourceHealth();
  const parts = describeSourceHealth(health);
  if (parts.length === 0) return null;
  const targetIssue = health.localSkills.length > 0 || health.outOfSyncTargets.length > 0;
  return (
    <button
      type="button"
      // The main view mirrors its path into the CLI Web UI: Sync for target drift, Git otherwise.
      onClick={() => navigate(targetIssue ? '/sync' : '/git')}
      className="flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] text-[var(--ink-2)] text-xs hover:bg-[var(--sel)] hover:text-[var(--ink)] transition-colors"
      title={parts.map((p) => p.detail).join('\n')}
      aria-label={`Source health: ${parts.map((p) => p.detail).join('; ')}`}
    >
      <HeartPulse size={13} />
      {parts.map((p) => p.label).join(' · ')}
    </button>
  );
}
