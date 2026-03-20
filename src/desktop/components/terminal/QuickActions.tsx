import { useState, useRef, useEffect } from 'react';
import { Command, Ellipsis } from 'lucide-react';
import { quickActionCommands, commandIconMap } from './skillshareCommands';

interface QuickActionsProps {
  onExecute: (command: string) => void;
  onOpenPalette: () => void;
}

export default function QuickActions({ onExecute, onOpenPalette }: QuickActionsProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [visibleCount, setVisibleCount] = useState(quickActionCommands.length);
  const [showOverflow, setShowOverflow] = useState(false);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver(() => {
      const width = container.clientWidth;
      // Each button ~70px icon-only, ~100px with label. Reserve ~60px for ⌘K + overflow.
      const available = width - 60;
      const perButton = width < 600 ? 32 : 80;
      const count = Math.max(1, Math.floor(available / perButton));
      setVisibleCount(Math.min(count, quickActionCommands.length));
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const visible = quickActionCommands.slice(0, visibleCount);
  const overflow = quickActionCommands.slice(visibleCount);
  const isCompact = visibleCount < quickActionCommands.length;

  return (
    <div ref={containerRef} className="flex items-center gap-1 relative">
      {visible.map((cmd) => {
        const Icon = commandIconMap[cmd.icon];
        return (
          <button
            key={cmd.name}
            type="button"
            onClick={() => onExecute(cmd.command)}
            className="flex items-center gap-1 px-2 py-1 text-[11px] text-pencil-light hover:text-pencil hover:bg-muted/30 rounded-[var(--radius-sm)] transition-colors"
            title={cmd.description}
          >
            {Icon && <Icon size={12} />}
            {!isCompact && <span>{cmd.label}</span>}
          </button>
        );
      })}
      {overflow.length > 0 && (
        <div className="relative">
          <button
            type="button"
            onClick={() => setShowOverflow((p) => !p)}
            className="flex items-center p-1 text-pencil-light hover:text-pencil hover:bg-muted/30 rounded-[var(--radius-sm)] transition-colors"
            title="More actions"
          >
            <Ellipsis size={14} />
          </button>
          {showOverflow && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowOverflow(false)} role="presentation" />
              <div className="absolute right-0 top-full mt-1 w-48 bg-paper border border-muted rounded-[var(--radius-md)] shadow-lg z-50 py-1">
                {overflow.map((cmd) => {
                  const Icon = commandIconMap[cmd.icon];
                  return (
                    <button
                      key={cmd.name}
                      type="button"
                      onClick={() => { onExecute(cmd.command); setShowOverflow(false); }}
                      className="w-full flex items-center gap-2 px-3 py-1.5 text-[11px] text-pencil-light hover:text-pencil hover:bg-muted/30 transition-colors text-left"
                    >
                      {Icon && <Icon size={12} />}
                      <span>{cmd.label}</span>
                    </button>
                  );
                })}
              </div>
            </>
          )}
        </div>
      )}
      <div className="w-px h-4 bg-muted/50 mx-1" />
      <button
        type="button"
        onClick={onOpenPalette}
        className="flex items-center gap-1 px-2 py-1 text-[11px] text-pencil-light hover:text-pencil hover:bg-muted/30 rounded-[var(--radius-sm)] transition-colors"
        title="Command Palette (Cmd+K)"
      >
        <Command size={12} />
        <span className="text-[10px] opacity-60">K</span>
      </button>
    </div>
  );
}
