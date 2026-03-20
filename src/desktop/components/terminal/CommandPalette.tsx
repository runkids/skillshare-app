import { useState, useRef, useEffect, useMemo } from 'react';
import { Search } from 'lucide-react';
import {
  skillshareCommands,
  commandIconMap,
  categoryLabels,
  type CommandCategory,
} from './skillshareCommands';
import type { SkillshareCommand } from './skillshareCommands';

interface CommandPaletteProps {
  onExecute: (command: string) => void;
  onClose: () => void;
}

type PaletteItem =
  | { type: 'header'; category: CommandCategory }
  | { type: 'command'; command: SkillshareCommand; flatIndex: number };

export default function CommandPalette({ onExecute, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [selectedFlatIndex, setSelectedFlatIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const { items, flatCommands } = useMemo(() => {
    const lower = query.toLowerCase();
    const filtered = query
      ? skillshareCommands.filter(
          (cmd) =>
            cmd.name.includes(lower) ||
            cmd.label.toLowerCase().includes(lower) ||
            cmd.description.toLowerCase().includes(lower) ||
            cmd.command.toLowerCase().includes(lower)
        )
      : skillshareCommands;

    // Group by category, preserving order
    const categoryOrder: CommandCategory[] = [
      'core', 'sync', 'skill-mgmt', 'backup', 'security', 'extras', 'other',
    ];
    const grouped = new Map<CommandCategory, SkillshareCommand[]>();
    for (const cmd of filtered) {
      const existing = grouped.get(cmd.category) ?? [];
      existing.push(cmd);
      grouped.set(cmd.category, existing);
    }

    const result: PaletteItem[] = [];
    const flat: SkillshareCommand[] = [];
    for (const cat of categoryOrder) {
      const cmds = grouped.get(cat);
      if (!cmds?.length) continue;
      result.push({ type: 'header', category: cat });
      for (const cmd of cmds) {
        result.push({ type: 'command', command: cmd, flatIndex: flat.length });
        flat.push(cmd);
      }
    }
    return { items: result, flatCommands: flat };
  }, [query]);

  useEffect(() => {
    setSelectedFlatIndex(0);
  }, [query]);

  // Scroll selected item into view
  useEffect(() => {
    const el = listRef.current?.querySelector('[data-selected="true"]');
    el?.scrollIntoView({ block: 'nearest' });
  }, [selectedFlatIndex]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedFlatIndex((i) => Math.min(i + 1, flatCommands.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedFlatIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && flatCommands[selectedFlatIndex]) {
      onExecute(flatCommands[selectedFlatIndex].command);
      onClose();
    }
  }

  function handleBackdropClick(e: React.MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] bg-black/50"
      onClick={handleBackdropClick}
      role="presentation"
    >
      <div className="w-[420px] bg-[#0f0f1a] border border-gray-700 rounded-lg shadow-2xl overflow-hidden">
        <div className="flex items-center gap-2 px-3 py-2.5 border-b border-gray-800">
          <Search size={14} className="text-gray-500 shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-transparent text-sm text-gray-200 outline-none placeholder:text-gray-600"
            placeholder="Type a skillshare command..."
          />
        </div>
        <div ref={listRef} className="max-h-[300px] overflow-y-auto py-1">
          {items.length === 0 ? (
            <div className="px-3 py-4 text-center text-xs text-gray-500">No matching commands</div>
          ) : (
            items.map((item, i) => {
              if (item.type === 'header') {
                return (
                  <div
                    key={`header-${item.category}`}
                    className="px-3 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-600"
                  >
                    {categoryLabels[item.category]}
                  </div>
                );
              }
              const { command: cmd, flatIndex } = item;
              const Icon = commandIconMap[cmd.icon];
              const isSelected = flatIndex === selectedFlatIndex;
              return (
                <button
                  key={cmd.name}
                  type="button"
                  data-selected={isSelected}
                  onClick={() => { onExecute(cmd.command); onClose(); }}
                  className={`w-full flex items-center gap-3 px-3 py-2 text-left transition-colors ${
                    isSelected ? 'bg-gray-800' : 'hover:bg-gray-800/50'
                  }`}
                >
                  <span className="text-gray-400 shrink-0">{Icon && <Icon size={14} />}</span>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm text-gray-200 font-medium">{cmd.label}</div>
                    <div className="text-xs text-gray-500 truncate">{cmd.description}</div>
                  </div>
                  <span className="text-[10px] text-gray-600 font-mono shrink-0">{cmd.command}</span>
                </button>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
