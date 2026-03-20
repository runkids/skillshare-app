// src/desktop/components/terminal/TerminalTab.tsx
import { memo } from 'react';
import { X } from 'lucide-react';
import type { TerminalSessionState } from '../../context/TerminalContext';

interface TerminalTabProps {
  session: TerminalSessionState;
  isActive: boolean;
  onClick: () => void;
  onClose: () => void;
}

const statusColors = {
  running: 'bg-green-500',
  completed: 'bg-gray-400',
  failed: 'bg-red-500',
};

export default memo(function TerminalTab({ session, isActive, onClick, onClose }: TerminalTabProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`group flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-[var(--radius-sm)] transition-colors whitespace-nowrap max-w-[160px] ${
        isActive
          ? 'bg-paper text-pencil shadow-sm'
          : 'text-pencil-light hover:text-pencil hover:bg-muted/30'
      }`}
    >
      <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${statusColors[session.status]}`} />
      <span className="truncate">{session.name}</span>
      {session.hasUnread && !isActive && (
        <span className="w-1.5 h-1.5 rounded-full bg-blue-500 shrink-0" />
      )}
      <span
        role="button"
        tabIndex={-1}
        onClick={(e) => { e.stopPropagation(); onClose(); }}
        onKeyDown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); onClose(); } }}
        className="ml-auto opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-muted/50 transition-opacity shrink-0"
      >
        <X size={12} />
      </span>
    </button>
  );
});
