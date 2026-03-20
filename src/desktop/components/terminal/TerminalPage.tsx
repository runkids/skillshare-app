import { useState, useCallback, useEffect } from 'react';
import { useTerminal } from '../../context/TerminalContext';
import TerminalToolbar from './TerminalToolbar';
import TerminalContainer from './TerminalContainer';
import TerminalStatusBar from './TerminalStatusBar';
import CommandPalette from './CommandPalette';

export default function TerminalPage() {
  const {
    sessions,
    activeSessionId,
    activeView,
    spawnSession,
    killSession,
    switchSession,
    executeInSession,
  } = useTerminal();

  const [showSearch, setShowSearch] = useState(false);
  const [showPalette, setShowPalette] = useState(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? null;

  const handleNewSession = useCallback(async () => {
    await spawnSession();
  }, [spawnSession]);

  const handleExecuteCommand = useCallback(
    async (command: string) => {
      if (activeSessionId) {
        const session = sessions.find((s) => s.id === activeSessionId);
        if (session?.status === 'running') {
          executeInSession(activeSessionId, command);
          return;
        }
      }
      // No active session or session dead -- spawn new one with command
      await spawnSession(command);
    },
    [activeSessionId, sessions, executeInSession, spawnSession]
  );

  // Keyboard shortcuts (only active when terminal view is shown)
  useEffect(() => {
    if (activeView !== 'terminal') return;

    function handleKeyDown(e: KeyboardEvent) {
      if (e.metaKey && e.key === 'k') {
        e.preventDefault();
        setShowPalette((prev) => !prev);
      } else if (e.metaKey && e.key === 't') {
        e.preventDefault();
        handleNewSession();
      } else if (e.metaKey && e.shiftKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        if (activeSessionId) killSession(activeSessionId);
      } else if (e.metaKey && e.shiftKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        setShowSearch((prev) => !prev);
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeView, activeSessionId, killSession, handleNewSession]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      <TerminalToolbar
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSwitchSession={switchSession}
        onCloseSession={killSession}
        onNewSession={handleNewSession}
        onExecuteCommand={handleExecuteCommand}
        onOpenPalette={() => setShowPalette(true)}
      />
      <TerminalContainer showSearch={showSearch} onCloseSearch={() => setShowSearch(false)} />
      <TerminalStatusBar session={activeSession} />
      {showPalette && (
        <CommandPalette onExecute={handleExecuteCommand} onClose={() => setShowPalette(false)} />
      )}
    </div>
  );
}
