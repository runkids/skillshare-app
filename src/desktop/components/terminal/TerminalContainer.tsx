// src/desktop/components/terminal/TerminalContainer.tsx
import { useRef, useEffect } from 'react';
import { useTerminal } from '../../context/TerminalContext';
import TerminalSearchBar from './TerminalSearchBar';

interface TerminalContainerProps {
  showSearch: boolean;
  onCloseSearch: () => void;
}

export default function TerminalContainer({ showSearch, onCloseSearch }: TerminalContainerProps) {
  const { activeSessionId, mountSession, unmountSession, getSessionLive } = useTerminal();
  const containerRef = useRef<HTMLDivElement>(null);
  const mountedSessionRef = useRef<string | null>(null);

  useEffect(() => {
    // Unmount previous session
    if (mountedSessionRef.current && mountedSessionRef.current !== activeSessionId) {
      unmountSession(mountedSessionRef.current);
      mountedSessionRef.current = null;
    }

    // Mount new session
    if (activeSessionId && containerRef.current) {
      // Clear container content safely before mounting new terminal
      while (containerRef.current.firstChild) {
        containerRef.current.removeChild(containerRef.current.firstChild);
      }
      mountSession(activeSessionId, containerRef.current);
      mountedSessionRef.current = activeSessionId;
    }

    return () => {
      if (mountedSessionRef.current) {
        unmountSession(mountedSessionRef.current);
        mountedSessionRef.current = null;
      }
    };
  }, [activeSessionId, mountSession, unmountSession]);

  const live = activeSessionId ? getSessionLive(activeSessionId) : undefined;
  const searchAddon = live?.handle?.searchAddon ?? null;

  return (
    <div className="flex-1 relative bg-[#030712] overflow-hidden">
      {showSearch && <TerminalSearchBar searchAddon={searchAddon} onClose={onCloseSearch} />}
      <div ref={containerRef} className="absolute inset-0 p-4 pt-4" />
      {!activeSessionId && (
        <div className="absolute inset-0 flex items-center justify-center text-gray-500 text-sm">
          Press + to start a new terminal session
        </div>
      )}
    </div>
  );
}
