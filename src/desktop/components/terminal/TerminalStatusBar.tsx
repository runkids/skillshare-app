import { useEffect } from 'react';
import {
  RefreshCw, TriangleAlert, CircleX, Package, Target, Clock,
} from 'lucide-react';
import type { TerminalSessionState } from '../../context/TerminalContext';
import { useSkillshareStatus } from '../../hooks/useSkillshareStatus';

interface TerminalStatusBarProps {
  session: TerminalSessionState | null;
  projectPath: string | null;
  activeView?: 'webui' | 'terminal';
}

const sessionStatusLabels = {
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
};

const sessionStatusColors = {
  running: 'text-green-400',
  completed: 'text-gray-400',
  failed: 'text-red-400',
};

function SyncIndicator({ syncStatus, pendingCount }: { syncStatus: string; pendingCount?: number }) {
  switch (syncStatus) {
    case 'synced':
      return <span className="flex items-center gap-1 text-green-400"><RefreshCw size={10} /> Synced</span>;
    case 'pending':
      return <span className="flex items-center gap-1 text-yellow-400"><TriangleAlert size={10} /> {pendingCount ?? ''} pending</span>;
    case 'error':
      return <span className="flex items-center gap-1 text-red-400"><CircleX size={10} /> Error</span>;
    default:
      return null;
  }
}

export default function TerminalStatusBar({ session, projectPath, activeView }: TerminalStatusBarProps) {
  const { status, error, refresh } = useSkillshareStatus(projectPath);

  // Refresh when terminal view gains focus
  useEffect(() => {
    if (activeView === 'terminal') refresh();
  }, [activeView, refresh]);

  // Hide entirely when there's nothing to show (no session, no status)
  if (!session && !status && !error) return null;

  return (
    <div className="h-6 flex items-center justify-between px-3 bg-[#0a0a0f] border-t border-gray-800 text-[11px] text-gray-500 shrink-0">
      <div className="flex items-center gap-3">
        {/* Skillshare status */}
        {error ? (
          <span className="flex items-center gap-1 text-yellow-400/70"><TriangleAlert size={10} /> {error}</span>
        ) : status ? (
          <>
            <SyncIndicator syncStatus={status.syncStatus} pendingCount={status.pendingCount} />
            <span className="flex items-center gap-1"><Package size={10} /> {status.skillsCount} skills</span>
            <span className="flex items-center gap-1"><Target size={10} /> {status.targetsCount} targets</span>
            {status.lastSyncTime && (
              <span className="flex items-center gap-1"><Clock size={10} /> {status.lastSyncTime}</span>
            )}
          </>
        ) : null}
      </div>
      <div className="flex items-center gap-3">
        {/* Session info */}
        {session && (
          <>
            <span className={sessionStatusColors[session.status]}>
              {sessionStatusLabels[session.status]}
            </span>
            {session.exitCode !== undefined && (
              <span>exit: {session.exitCode}</span>
            )}
            <span className="truncate max-w-[300px]">{session.projectPath}</span>
          </>
        )}
      </div>
    </div>
  );
}
