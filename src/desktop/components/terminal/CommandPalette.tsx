// src/desktop/components/terminal/CommandPalette.tsx
import { useState, useRef, useEffect, useMemo } from 'react';
import {
  RefreshCw, ShieldCheck, Activity, Stethoscope, Target, List,
  FolderPlus, Plus, Minus, GitCompare, History, Settings, Search,
} from 'lucide-react';
import { skillshareCommands } from './skillshareCommands';

const iconMap: Record<string, React.ComponentType<{ size?: number }>> = {
  RefreshCw, ShieldCheck, Activity, Stethoscope, Target, List,
  FolderPlus, Plus, Minus, GitCompare, History, Settings,
};

interface CommandPaletteProps {
  onExecute: (command: string) => void;
  onClose: () => void;
}

export default function CommandPalette({ onExecute, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const filtered = useMemo(() => {
    if (!query) return skillshareCommands;
    const lower = query.toLowerCase();
    return skillshareCommands.filter(
      cmd => cmd.name.includes(lower) || cmd.label.toLowerCase().includes(lower) || cmd.description.toLowerCase().includes(lower)
    );
  }, [query]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(i => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(i => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      onExecute(filtered[selectedIndex].command);
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
      onKeyDown={() => {}}
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
        <div className="max-h-[300px] overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-3 py-4 text-center text-xs text-gray-500">No matching commands</div>
          ) : (
            filtered.map((cmd, i) => {
              const Icon = iconMap[cmd.icon];
              return (
                <button
                  key={cmd.name}
                  type="button"
                  onClick={() => { onExecute(cmd.command); onClose(); }}
                  className={`w-full flex items-center gap-3 px-3 py-2 text-left transition-colors ${
                    i === selectedIndex ? 'bg-gray-800' : 'hover:bg-gray-800/50'
                  }`}
                >
                  <span className="text-gray-400 shrink-0">
                    {Icon && <Icon size={14} />}
                  </span>
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
