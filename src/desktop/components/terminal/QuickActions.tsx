// src/desktop/components/terminal/QuickActions.tsx
import {
  RefreshCw, ShieldCheck, Activity, Stethoscope, Target, List, Command,
} from 'lucide-react';
import { quickActionCommands } from './skillshareCommands';

const iconMap: Record<string, React.ComponentType<{ size?: number }>> = {
  RefreshCw, ShieldCheck, Activity, Stethoscope, Target, List,
};

interface QuickActionsProps {
  onExecute: (command: string) => void;
  onOpenPalette: () => void;
}

export default function QuickActions({ onExecute, onOpenPalette }: QuickActionsProps) {
  return (
    <div className="flex items-center gap-1">
      {quickActionCommands.map((cmd) => {
        const Icon = iconMap[cmd.icon];
        return (
          <button
            key={cmd.name}
            type="button"
            onClick={() => onExecute(cmd.command)}
            className="flex items-center gap-1 px-2 py-1 text-[11px] text-pencil-light hover:text-pencil hover:bg-muted/30 rounded-[var(--radius-sm)] transition-colors"
            title={cmd.description}
          >
            {Icon && <Icon size={12} />}
            {cmd.label}
          </button>
        );
      })}
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
