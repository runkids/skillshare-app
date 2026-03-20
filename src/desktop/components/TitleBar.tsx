import { useState, useEffect } from 'react';
import { Settings } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import ProjectDropdown from './ProjectDropdown';

export default function TitleBar() {
  const navigate = useNavigate();
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    // Check initial state
    appWindow.isFullscreen().then(setIsFullscreen);
    // Listen for changes
    const unlisten = appWindow.onResized(() => {
      appWindow.isFullscreen().then(setIsFullscreen);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return (
    <div
      data-tauri-drag-region
      className="h-12 flex items-center justify-between px-4 bg-paper border-b border-muted select-none shrink-0"
      style={{ paddingLeft: isFullscreen ? '16px' : '80px' }}
    >
      <ProjectDropdown />
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
