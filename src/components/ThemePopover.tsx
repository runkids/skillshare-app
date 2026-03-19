import { useState, useRef, useEffect, useLayoutEffect, useCallback } from 'react';
import { Palette } from 'lucide-react';
import { useTheme, type ModePreference } from '../context/ThemeContext';
import { shadows } from '../design';

const modes: { value: ModePreference; label: string }[] = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'system', label: 'System' },
];

export default function ThemePopover() {
  const { modePreference, setModePreference } = useTheme();
  const [open, setOpen] = useState(false);
  const [dropUp, setDropUp] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Return focus to trigger on close
  const prevOpen = useRef(open);
  useEffect(() => {
    if (prevOpen.current && !open) {
      triggerRef.current?.focus();
    }
    prevOpen.current = open;
  }, [open]);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [open]);

  // Collision detection
  useLayoutEffect(() => {
    if (!open || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const panelHeight = 120;
    setDropUp(rect.top > panelHeight);
  }, [open]);

  // Focus first radio on open
  useEffect(() => {
    if (!open || !panelRef.current) return;
    const firstRadio = panelRef.current.querySelector('[role="radio"]') as HTMLElement;
    firstRadio?.focus();
  }, [open]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
      e.preventDefault();

      const idx = modes.findIndex((i) => i.value === modePreference);
      const next =
        e.key === 'ArrowRight'
          ? modes[(idx + 1) % modes.length]
          : modes[(idx - 1 + modes.length) % modes.length];
      setModePreference(next.value);
    },
    [modePreference, setModePreference]
  );

  return (
    <div ref={containerRef} className="relative">
      <button
        ref={triggerRef}
        onClick={() => setOpen(!open)}
        className="flex items-center gap-3 px-3 py-1.5 text-sm text-pencil-light hover:text-pencil hover:bg-muted/20 transition-colors cursor-pointer w-full"
        aria-label="Theme settings"
        aria-expanded={open}
      >
        <Palette size={16} strokeWidth={2.5} />
        Theme
      </button>

      {open && (
        <div
          ref={panelRef}
          role="dialog"
          aria-label="Theme settings"
          className={`
            absolute left-0 z-50 w-56 bg-surface border border-muted p-3 rounded-[var(--radius-md)] animate-dropdown-in
            ${dropUp ? 'bottom-full mb-2' : 'top-full mt-2'}
          `}
          style={{ boxShadow: shadows.lg }}
        >
          <div role="radiogroup" aria-label="Mode">
            <div className="text-xs font-medium text-muted-dark uppercase tracking-wider mb-2">
              Mode
            </div>
            <div className="flex gap-1.5">
              {modes.map((m) => (
                <button
                  key={m.value}
                  role="radio"
                  aria-checked={modePreference === m.value}
                  onClick={() => setModePreference(m.value)}
                  onKeyDown={handleKeyDown}
                  className={`
                    flex-1 px-2 py-1.5 text-xs rounded-lg transition-colors cursor-pointer
                    focus-visible:ring-2 focus-visible:ring-pencil/20 focus-visible:outline-none
                    ${
                      modePreference === m.value
                        ? 'bg-pencil text-paper font-medium'
                        : 'bg-muted/30 text-pencil-light hover:bg-muted/50'
                    }
                  `}
                  tabIndex={modePreference === m.value ? 0 : -1}
                >
                  {m.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
