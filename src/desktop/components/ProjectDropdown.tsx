import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Globe, Folder, Settings } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useProjects } from '../context/ProjectContext';

export default function ProjectDropdown() {
  const { projects, activeProject, switchWithRestart } = useProjects();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  // Close on click outside — only listen when dropdown is open
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const globalProjects = projects.filter((p) => p.projectType === 'global');
  const localProjects = projects.filter((p) => p.projectType === 'project');

  const handleSwitch = async (id: string, e: React.MouseEvent) => {
    setOpen(false);
    if (id === activeProject?.id) return;
    await switchWithRestart(id, { newSession: e.altKey });
  };

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-sm)] hover:bg-muted/50 transition-colors text-sm font-medium text-pencil"
      >
        {activeProject?.projectType === 'global' ? (
          <Globe size={14} strokeWidth={1.75} />
        ) : (
          <Folder size={14} strokeWidth={1.75} />
        )}
        <span className="max-w-[160px] truncate">{activeProject?.name || 'No Project'}</span>
        <ChevronDown size={12} strokeWidth={1.75} />
      </button>

      {open && (
        <div className="absolute top-full right-0 mt-1 w-56 bg-surface border-[length:var(--bw)] border-[var(--line-2)] rounded-[var(--r-box)] shadow-lg z-50 py-1">
          {globalProjects.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={(e) => handleSwitch(p.id, e)}
              className={`w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-muted/30 transition-colors ${
                p.id === activeProject?.id ? 'text-pencil font-medium' : 'text-pencil-light'
              }`}
            >
              <Globe size={14} strokeWidth={1.75} className="shrink-0" />
              <span className="truncate">{p.name}</span>
            </button>
          ))}
          {globalProjects.length > 0 && localProjects.length > 0 && (
            <div className="border-t border-muted my-1" />
          )}
          {localProjects.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={(e) => handleSwitch(p.id, e)}
              className={`w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-muted/30 transition-colors ${
                p.id === activeProject?.id ? 'text-pencil font-medium' : 'text-pencil-light'
              }`}
            >
              <Folder size={14} strokeWidth={1.75} className="shrink-0" />
              <span className="truncate">{p.name}</span>
            </button>
          ))}
          <div className="border-t border-muted my-1" />
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              navigate('/settings?tab=projects');
            }}
            className="w-full flex items-center gap-2 px-3 py-2 text-sm text-pencil-light hover:bg-muted/30 transition-colors"
          >
            <Settings size={14} strokeWidth={1.75} className="shrink-0" />
            <span>Manage Projects</span>
          </button>
        </div>
      )}
    </div>
  );
}
