import { useState, useEffect } from 'react';
import { Settings, Monitor, TerminalSquare } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import ProjectDropdown from './ProjectDropdown';
import { useTerminal } from '../context/TerminalContext';

// TODO: re-enable Web UI / Terminal tab switcher when terminal feature is ready
const SHOW_VIEW_TABS = false;

export default function TitleBar() {
  const navigate = useNavigate();
  const { activeView, setActiveView, hasUnreadAny } = useTerminal();
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    appWindow.isFullscreen().then(setIsFullscreen);
    const unlisten = appWindow.onResized(() => {
      appWindow.isFullscreen().then(setIsFullscreen);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!SHOW_VIEW_TABS) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.metaKey && e.key === '1') {
        e.preventDefault();
        setActiveView('webui');
      } else if (e.metaKey && e.key === '2') {
        e.preventDefault();
        setActiveView('terminal');
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [setActiveView]);

  return (
    <div
      data-tauri-drag-region
      className="h-12 flex items-center justify-between px-4 bg-paper border-b border-muted select-none shrink-0"
      style={{ paddingLeft: isFullscreen ? '16px' : '80px' }}
    >
      <div className="flex items-center gap-3">
        <ProjectDropdown />
        {SHOW_VIEW_TABS && (
          <div className="flex items-center bg-muted/30 rounded-[var(--radius-sm)] p-0.5">
            <button
              type="button"
              onClick={() => setActiveView('webui')}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-sm)] text-xs font-medium transition-colors ${
                activeView === 'webui'
                  ? 'bg-paper text-pencil shadow-sm'
                  : 'text-pencil-light hover:text-pencil'
              }`}
            >
              <Monitor size={13} />
              Web UI
            </button>
            <button
              type="button"
              onClick={() => setActiveView('terminal')}
              className={`relative flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-sm)] text-xs font-medium transition-colors ${
                activeView === 'terminal'
                  ? 'bg-paper text-pencil shadow-sm'
                  : 'text-pencil-light hover:text-pencil'
              }`}
            >
              <TerminalSquare size={13} />
              Terminal
              {hasUnreadAny && activeView !== 'terminal' && (
                <span className="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-blue-500" />
              )}
            </button>
          </div>
        )}
      </div>
      <button
        type="button"
        onClick={() => navigate('/settings')}
        className="p-1.5 rounded-[var(--radius-sm)] hover:bg-muted/50 transition-colors text-pencil-light hover:text-pencil"
        title="Settings"
      >
        <Settings size={16} strokeWidth={2.5} />
      </button>
    </div>
  );
}
