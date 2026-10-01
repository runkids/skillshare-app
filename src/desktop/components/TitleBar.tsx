import { useEffect } from 'react';
import {
  Settings,
  Monitor,
  TerminalSquare,
  RotateCw,
  History,
  Command,
  FileCog,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import ProjectDropdown from './ProjectDropdown';
import { useTerminal } from '../context/TerminalContext';
import { useProjects } from '../context/ProjectContext';
import { isMacOS } from '../utils/platform';
import { useUpdates } from '../hooks/useUpdates';
import { useFullscreen } from '../hooks/useFullscreen';
import StatusButton from './status/StatusButton';
import ServerStatus from './ServerStatus';
import QuickSyncButton from './QuickSyncButton';
import { tauriBridge } from '../api/tauri-bridge';
import appIcon from '../../../src-tauri/icons/64x64.png';

// TODO: re-enable Web UI / Terminal tab switcher when terminal feature is ready
const SHOW_VIEW_TABS = false;

export default function TitleBar() {
  const navigate = useNavigate();
  const updates = useUpdates();
  const hasUpdate = Boolean(updates.app || updates.cli);
  const { activeView, setActiveView, hasUnreadAny } = useTerminal();
  const { switching, reloadView } = useProjects();
  // macOS draws native traffic lights over the window (titleBarStyle: Overlay),
  // except in full screen, where they are hidden.
  const fullscreen = useFullscreen();
  const mac = isMacOS() && !fullscreen;
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
      <StatusButton />
      <ServerStatus />
      <QuickSyncButton disabled={switching} />
      <button
        type="button"
        onClick={() => void tauriBridge.openQuickActions().catch(() => {})}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)]"
        title="Quick Actions"
        aria-label="Quick Actions"
      >
        <Command size={15} />
      </button>
      <button
        type="button"
        onClick={() => navigate('/config')}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)]"
        title="Config files"
        aria-label="Config files"
      >
        <FileCog size={16} />
      </button>
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
        onClick={() => navigate('/activity')}
        className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)]"
        title="Activity"
        aria-label="Activity"
      >
        <History size={16} />
      </button>
      <button
        type="button"
        onClick={() =>
          navigate(hasUpdate ? `/settings?tab=${updates.app ? 'about' : 'cli'}` : '/settings')
        }
        className="relative w-[30px] h-[30px] flex items-center justify-center rounded-[var(--r-ctl)] hover:bg-[var(--sel)] transition-colors text-[var(--ink-2)] hover:text-[var(--ink)]"
        title={hasUpdate ? 'Settings — update available' : 'Settings'}
        aria-label={hasUpdate ? 'Settings, update available' : 'Settings'}
      >
        <Settings size={16} />
        {hasUpdate && (
          <span className="absolute top-1 right-1 w-2 h-2 rounded-full bg-[var(--accent)]" />
        )}
      </button>
    </div>
  );
}
