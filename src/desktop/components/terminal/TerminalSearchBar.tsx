// src/desktop/components/terminal/TerminalSearchBar.tsx
import { useState, useRef, useEffect } from 'react';
import { Search, ChevronUp, ChevronDown, X } from 'lucide-react';
import type { SearchAddon } from '@xterm/addon-search';

interface TerminalSearchBarProps {
  searchAddon: SearchAddon | null;
  onClose: () => void;
}

export default function TerminalSearchBar({ searchAddon, onClose }: TerminalSearchBarProps) {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!searchAddon || !query) return;
    searchAddon.findNext(query, { decorations: {
      matchOverviewRuler: '#eab308',
      activeMatchColorOverviewRuler: '#f59e0b',
      matchBackground: '#854d0e40',
      activeMatchBackground: '#eab30880',
    }});
  }, [query, searchAddon]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Escape') {
      searchAddon?.clearDecorations();
      onClose();
    } else if (e.key === 'Enter' && e.shiftKey) {
      searchAddon?.findPrevious(query);
    } else if (e.key === 'Enter') {
      searchAddon?.findNext(query);
    }
  }

  return (
    <div className="absolute top-1 right-2 z-10 flex items-center gap-1 bg-[#1a1a2e] border border-gray-700 rounded px-2 py-1 shadow-lg">
      <Search size={13} className="text-gray-500 shrink-0" />
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        className="bg-transparent text-gray-200 text-xs outline-none w-40 placeholder:text-gray-600"
        placeholder="Search..."
      />
      <button type="button" onClick={() => searchAddon?.findPrevious(query)} className="p-0.5 text-gray-400 hover:text-gray-200">
        <ChevronUp size={13} />
      </button>
      <button type="button" onClick={() => searchAddon?.findNext(query)} className="p-0.5 text-gray-400 hover:text-gray-200">
        <ChevronDown size={13} />
      </button>
      <button type="button" onClick={() => { searchAddon?.clearDecorations(); onClose(); }} className="p-0.5 text-gray-400 hover:text-gray-200">
        <X size={13} />
      </button>
    </div>
  );
}
