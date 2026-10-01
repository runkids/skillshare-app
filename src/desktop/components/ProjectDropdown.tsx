import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Check, ChevronsUpDown, Globe, Folder, List, Plus } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useProjects } from '../context/ProjectContext';
import type { Project } from '../api/tauri-bridge';
import { isMacOS } from '../utils/platform';
import { shortPath } from '../utils/path';

// Display-only: shorten the home directory to ~ for the mono path labels.

const CLOSE_MS = 130;

const prefersReducedMotion = () =>
  window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

function ProjectIcon({ project, size }: { project: Project | null; size: number }) {
  const Icon = project?.projectType === 'global' ? Globe : Folder;
  return <Icon size={size} />;
}

export default function ProjectDropdown() {
  const { projects, activeProject, switchWithRestart } = useProjects();
  // 'closing' keeps the menu mounted while the exit animation plays.
  const [menu, setMenu] = useState<'open' | 'closing' | 'closed'>('closed');
  const open = menu === 'open';
  const ref = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  const close = useCallback(
    () => setMenu((m) => (m === 'closed' || prefersReducedMotion() ? 'closed' : 'closing')),
    []
  );

  // Fallback in case animationend never fires (e.g. the element is hidden).
  useEffect(() => {
    if (menu !== 'closing') return;
    const t = setTimeout(() => setMenu('closed'), CLOSE_MS + 50);
    return () => clearTimeout(t);
  }, [menu]);

  // Close on click outside or Escape — only listen when dropdown is open.
  // Clicks inside the CLI iframe never reach this document; they blur the window instead.
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('mousedown', handler);
    document.addEventListener('keydown', keyHandler);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('mousedown', handler);
      document.removeEventListener('keydown', keyHandler);
      window.removeEventListener('blur', close);
    };
  }, [open, close]);

  const globalProjects = projects.filter((p) => p.projectType === 'global');
  const localProjects = projects.filter((p) => p.projectType === 'project');

  const handleSwitch = async (id: string, e: React.MouseEvent) => {
    close();
    if (id === activeProject?.id) return;
    await switchWithRestart(id, { newSession: e.altKey });
  };

  const goToProjectSettings = () => {
    close();
    navigate('/settings?tab=projects');
  };

  const renderRow = (p: Project) => {
    const active = p.id === activeProject?.id;
    return (
      <button
        key={p.id}
        type="button"
        role="menuitem"
        onClick={(e) => handleSwitch(p.id, e)}
        className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded-[var(--r-ctl)] text-left text-[var(--ink)] transition-colors ${
          active ? 'bg-[var(--sel)]' : 'hover:bg-[var(--sel)]'
        }`}
      >
        <span className="w-[26px] h-[26px] shrink-0 rounded-[7px] bg-[var(--accent-bg)] text-[var(--accent)] flex items-center justify-center">
          <ProjectIcon project={p} size={14} />
        </span>
        <span className="flex-1 min-w-0 flex flex-col">
          <span className="text-[13px] font-semibold truncate">{p.name}</span>
          <span className="font-mono text-[11px] text-[var(--ink-3)] truncate">
            {shortPath(p.path)}
          </span>
        </span>
        {active && <Check size={16} className="text-[var(--accent)]" aria-label="Active" />}
      </button>
    );
  };

  const sectionLabel = 'text-[11px] font-semibold tracking-[.06em] text-[var(--ink-3)] px-2.5';
  const divider = <div className="h-px bg-[var(--line)] mx-1 my-1.5" />;

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => (open ? close() : setMenu('open'))}
        className={`h-[30px] flex items-center gap-2 pl-1.5 pr-2.5 rounded-[var(--r-ctl)] border transition-colors text-[var(--ink)] ${
          open
            ? 'bg-[var(--surface)] border-[var(--line-2)]'
            : 'border-transparent hover:bg-[var(--sel)]'
        }`}
      >
        <span className="w-5 h-5 rounded-[6px] bg-[var(--accent-bg)] text-[var(--accent)] flex items-center justify-center">
          <ProjectIcon project={activeProject} size={13} />
        </span>
        <span className="text-[13px] font-semibold max-w-[160px] truncate">
          {activeProject?.name || 'No Project'}
        </span>
        {activeProject && (
          <span className="font-mono text-[11px] text-[var(--ink-3)] max-w-[200px] truncate">
            {shortPath(activeProject.path)}
          </span>
        )}
        <ChevronsUpDown size={13} className="text-[var(--ink-3)]" />
      </button>

      {menu !== 'closed' && (
        <div
          role="menu"
          aria-label="Projects"
          onAnimationEnd={() => setMenu((m) => (m === 'closing' ? 'closed' : m))}
          className={`absolute top-full left-0 mt-1.5 w-[300px] p-1.5 bg-[var(--surface)] rounded-[var(--r-box)] shadow-[var(--sh-float)] z-50 origin-top-left ${
            open
              ? 'motion-safe:animate-[dialogIn_150ms_ease-out]'
              : 'pointer-events-none motion-safe:animate-[dialogIn_130ms_ease-in_reverse_forwards]'
          }`}
        >
          {globalProjects.length > 0 && (
            <>
              <div className={`${sectionLabel} pt-2 pb-1`}>GLOBAL</div>
              {globalProjects.map(renderRow)}
              {divider}
            </>
          )}
          <div className={`${sectionLabel} py-1`}>PROJECTS</div>
          {localProjects.map(renderRow)}
          {localProjects.length === 0 && (
            <p className="px-2.5 pt-1.5 pb-2 text-xs text-[var(--ink-3)]">
              No projects yet. Add a repo to give it its own skills.
            </p>
          )}
          {divider}
          <button
            type="button"
            role="menuitem"
            onClick={goToProjectSettings}
            className="w-full flex items-center gap-2.5 h-[34px] px-2.5 rounded-[var(--r-ctl)] text-[13px] text-[var(--ink)] hover:bg-[var(--sel)] transition-colors"
          >
            <Plus size={16} />
            Add project folder…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={goToProjectSettings}
            className="w-full flex items-center gap-2.5 h-[34px] px-2.5 rounded-[var(--r-ctl)] text-[13px] text-[var(--ink-2)] hover:bg-[var(--sel)] transition-colors"
          >
            <List size={16} />
            Manage projects
          </button>
          <p className="mt-1.5 mx-1 pt-2 px-1.5 pb-1 border-t border-[var(--line)] text-[11px] text-[var(--ink-3)]">
            <kbd className="font-mono px-[5px] py-px rounded-[4px] bg-[var(--sunken)] text-[var(--ink-2)]">
              {isMacOS() ? '⌥' : 'Alt'}
            </kbd>{' '}
            + click a project to open it in a new session
          </p>
        </div>
      )}
    </div>
  );
}
