/**
 * Terminal Tab Component - Memoized tab for PTY sessions
 * Prevents unnecessary re-renders when other session states change
 */

import React from 'react';
import { X } from 'lucide-react';
import { Button } from '../ui/Button';
import type { PtySession } from '../../hooks/usePtySessions';

interface TerminalTabProps {
  session: PtySession;
  isActive: boolean;
  port?: number;
  onSelect: (sessionId: string) => void;
  onClose: (sessionId: string) => void;
}

// Status color mapping
const statusColors: Record<PtySession['status'], string> = {
  running: 'bg-yellow-400',
  completed: 'bg-green-400',
  failed: 'bg-red-400',
};

export const TerminalTab = React.memo(function TerminalTab({
  session,
  isActive,
  port,
  onSelect,
  onClose,
}: TerminalTabProps) {
  // Generate display label
  const projectLabel =
    session.projectName || session.projectPath.split(/[\\/]/).filter(Boolean).pop();
  const displayName = projectLabel ? `${projectLabel}: ${session.name}` : session.name;

  return (
    <div
      onClick={() => onSelect(session.id)}
      className={`group flex items-center gap-1.5 px-2 py-1 rounded text-xs cursor-pointer ${
        isActive ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:bg-accent'
      }`}
    >
      {/* Status indicator */}
      <span className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${statusColors[session.status]}`} />

      {/* Session name */}
      <span className="truncate max-w-[150px]">{displayName}</span>

      {/* Port indicator */}
      {port && <span className="text-yellow-400 text-[10px] flex-shrink-0">:{port}</span>}

      {/* Close button */}
      <Button
        variant="ghost"
        size="icon"
        onClick={(e) => {
          e.stopPropagation();
          onClose(session.id);
        }}
        className="opacity-0 group-hover:opacity-100 h-auto p-0.5"
      >
        <X className="w-3 h-3" />
      </Button>
    </div>
  );
});

export default TerminalTab;
