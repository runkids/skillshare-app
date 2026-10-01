import { useEffect, useRef, useState } from 'react';
import { ArrowUpCircle } from 'lucide-react';
import { Link } from 'react-router-dom';
import type { AvailableUpdates } from '../api/tauri-bridge';

/** One count for resource updates, with links to each kind's CLI review page. */
export default function ResourceUpdatesBadge({ updates }: { updates: AvailableUpdates }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const groups = [
    { names: updates.skills, one: 'skill', many: 'skills', path: '/skills?tab=updates' },
    { names: updates.plugins, one: 'plugin', many: 'plugins', path: '/plugins' },
    { names: updates.agents, one: 'agent', many: 'agents', path: '/agents?tab=updates' },
    {
      names: updates.repositories,
      one: 'repository',
      many: 'repositories',
      path: '/skills?tab=updates',
    },
  ].filter((group) => group.names.length > 0);
  const count = groups.reduce((total, group) => total + group.names.length, 0);
  const summary = groups
    .map((group) => `${group.names.length} ${group.names.length === 1 ? group.one : group.many}`)
    .join(', ');

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) close();
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', keydown);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('keydown', keydown);
      window.removeEventListener('blur', close);
    };
  }, [open]);

  if (count === 0) return null;

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-controls="resource-updates"
        className="flex items-center gap-1.5 h-[26px] px-2.5 rounded-[var(--r-btn)] bg-[var(--sel)] text-[var(--accent)] text-xs hover:text-[var(--ink)] transition-colors"
        title={`${summary} ${count === 1 ? 'has' : 'have'} updates`}
      >
        <ArrowUpCircle size={13} />
        {count} update{count === 1 ? '' : 's'}
      </button>
      {open && (
        <div
          id="resource-updates"
          className="absolute right-0 top-full mt-2 z-50 w-72 p-1 rounded-[var(--r-box)] border border-[var(--line)] bg-[var(--surface)] shadow-lg"
        >
          {groups.map((group) => (
            <Link
              key={group.many}
              to={group.path}
              onClick={() => setOpen(false)}
              className="block px-3 py-2 rounded-[var(--r-btn)] hover:bg-[var(--sel)] text-xs text-[var(--ink)]"
            >
              <span className="font-medium">
                {group.names.length} {group.names.length === 1 ? group.one : group.many}
              </span>
              <span className="block mt-1 text-[var(--ink-2)] break-words">
                {group.names.join(', ')}
              </span>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
