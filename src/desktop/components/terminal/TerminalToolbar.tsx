// src/desktop/components/terminal/TerminalToolbar.tsx
import { Plus } from 'lucide-react';
import TerminalTab from './TerminalTab';
import QuickActions from './QuickActions';
import type { TerminalSessionState } from '../../context/TerminalContext';

interface TerminalToolbarProps {
  sessions: TerminalSessionState[];
  activeSessionId: string | null;
  onSwitchSession: (id: string) => void;
  onCloseSession: (id: string) => void;
  onNewSession: () => void;
  onExecuteCommand: (command: string) => void;
  onOpenPalette: () => void;
}

export default function TerminalToolbar({
  sessions,
  activeSessionId,
  onSwitchSession,
  onCloseSession,
  onNewSession,
  onExecuteCommand,
  onOpenPalette,
}: TerminalToolbarProps) {
  return (
    <div className="flex items-center justify-between px-2 py-1 bg-paper border-b border-muted shrink-0">
      <div className="flex items-center gap-1 overflow-x-auto scrollbar-none">
        {sessions.map((s) => (
          <TerminalTab
            key={s.id}
            session={s}
            isActive={s.id === activeSessionId}
            onClick={() => onSwitchSession(s.id)}
            onClose={() => onCloseSession(s.id)}
          />
        ))}
        <button
          type="button"
          onClick={onNewSession}
          className="p-1.5 text-pencil-light hover:text-pencil hover:bg-muted/30 rounded-[var(--radius-sm)] transition-colors shrink-0"
          title="New terminal (Cmd+T)"
        >
          <Plus size={14} />
        </button>
      </div>
      <QuickActions onExecute={onExecuteCommand} onOpenPalette={onOpenPalette} />
    </div>
  );
}
