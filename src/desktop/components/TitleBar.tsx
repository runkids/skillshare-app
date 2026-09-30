import { useEffect } from 'react';
import { Settings, Monitor, TerminalSquare, RotateCw } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import ProjectDropdown from './ProjectDropdown';
import { useTerminal } from '../context/TerminalContext';
import { useTauri } from '../context/TauriContext';
import { useProjects } from '../context/ProjectContext';
import { isMacOS } from '../utils/platform';
import appIcon from '../../../src-tauri/icons/64x64.png';

// TODO: re-enable Web UI / Terminal tab switcher when terminal feature is ready
const SHOW_VIEW_TABS = false;

export default function TitleBar() {
  const navigate = useNavigate();
  const { activeView, setActiveView, hasUnreadAny } = useTerminal();
  const { appInfo } = useTauri();
  const { switching, reloadView } = useProjects();
  // macOS draws native traffic lights over the window (titleBarStyle: Overlay).
  const mac = isMacOS();
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
      className="h-11 flex items-center gap-2 pr-3 bg-[var(--side)] border-b border-[var(--line)] select-none shrink-0"
      style={{ paddingLeft: mac ? 84 : 12 }}
    >
      {!mac && (
        <span
          data-testid="app-mark"
          className="flex items-center gap-2 h-5 pr-2.5 mr-0.5 border-r border-[var(--line)]"
        >
          <img src={appIcon} alt="" className="w-5 h-5" draggable={false} />
          <span className="text-[13px] font-bold tracking-[-0.01em] text-[var(--ink)]">
            skillshare
          </span>
        </span>
      )}
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
      <div data-tauri-drag-region className="flex-1 self-stretch" />
      {appInfo?.serverRunning && (
        <span className="flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] bg-[var(--ok-bg)] text-[var(--ok)] text-xs">
          <span className="w-1.5 h-1.5 rounded-full bg-[var(--ok)]" />
          Server running
        </span>
      )}
      <button
        type="button"
        onClick={() => {
          reloadView();
          navigate('/');
        }}
        disabled={switching}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)] disabled:opacity-50"
        title="Reload"
        aria-label="Reload"
      >
        <RotateCw size={15} className={switching ? 'motion-safe:animate-spin' : undefined} />
      </button>
      <button
        type="button"
        onClick={() => navigate('/settings')}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)]"
        title="Settings"
        aria-label="Settings"
      >
        <Settings size={16} />
      </button>
    </div>
  );
}
