// src/desktop/components/terminal/TerminalStatusBar.tsx
import type { TerminalSessionState } from '../../context/TerminalContext';

interface TerminalStatusBarProps {
  session: TerminalSessionState | null;
}

const statusLabels = {
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
};

const statusColors = {
  running: 'text-green-400',
  completed: 'text-gray-400',
  failed: 'text-red-400',
};

export default function TerminalStatusBar({ session }: TerminalStatusBarProps) {
  if (!session) return null;

  return (
    <div className="h-6 flex items-center justify-between px-3 bg-[#0a0a0f] border-t border-gray-800 text-[11px] text-gray-500 shrink-0">
      <div className="flex items-center gap-2">
        <span className={statusColors[session.status]}>
          {statusLabels[session.status]}
        </span>
        {session.exitCode !== undefined && (
          <span>exit: {session.exitCode}</span>
        )}
      </div>
      <div className="flex items-center gap-2">
        <span className="truncate max-w-[300px]">{session.projectPath}</span>
      </div>
    </div>
  );
}
