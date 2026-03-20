import {
  RefreshCw,
  ShieldCheck,
  Activity,
  Stethoscope,
  Target,
  List,
  FolderPlus,
  Plus,
  Minus,
  GitCompare,
  History,
  Settings,
  Upload,
  Download,
  Search,
  PackagePlus,
  PackageMinus,
  FilePlus,
  CircleArrowUp,
  RotateCcw,
  Archive,
  Trash2,
  Eye,
  Globe,
  Terminal,
  MonitorPlay,
  Info,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export type CommandCategory = 'core' | 'sync' | 'skill-mgmt' | 'backup' | 'security' | 'extras' | 'other';

export interface SkillshareCommand {
  name: string;
  label: string;
  description: string;
  command: string;
  icon: string;
  category: CommandCategory;
}

export const commandIconMap: Record<string, LucideIcon> = {
  RefreshCw, ShieldCheck, Activity, Stethoscope, Target, List,
  FolderPlus, Plus, Minus, GitCompare, History, Settings,
  Upload, Download, Search, PackagePlus, PackageMinus, FilePlus,
  CircleArrowUp, RotateCcw, Archive, Trash2, Eye, Globe,
  Terminal, MonitorPlay, Info,
};

export const categoryLabels: Record<CommandCategory, string> = {
  core: 'Core',
  sync: 'Sync',
  'skill-mgmt': 'Skill Management',
  backup: 'Backup & Restore',
  security: 'Security & Diagnostics',
  extras: 'Extras',
  other: 'Other',
};

export const skillshareCommands: SkillshareCommand[] = [
  // Core
  { name: 'init', label: 'Init', description: 'Initialize a new project', command: 'skillshare init', icon: 'FolderPlus', category: 'core' },
  { name: 'install', label: 'Install', description: 'Add a skill from a repo or path', command: 'skillshare install', icon: 'PackagePlus', category: 'core' },
  { name: 'uninstall', label: 'Uninstall', description: 'Remove a skill', command: 'skillshare uninstall', icon: 'PackageMinus', category: 'core' },
  { name: 'add', label: 'Add', description: 'Add files to tracking', command: 'skillshare add', icon: 'Plus', category: 'core' },
  { name: 'remove', label: 'Remove', description: 'Remove files from tracking', command: 'skillshare remove', icon: 'Minus', category: 'core' },
  { name: 'list', label: 'List', description: 'List all skills', command: 'skillshare list', icon: 'List', category: 'core' },
  { name: 'search', label: 'Search', description: 'Search for skills', command: 'skillshare search', icon: 'Search', category: 'core' },

  // Sync
  { name: 'sync', label: 'Sync', description: 'Push skills to all targets', command: 'skillshare sync', icon: 'RefreshCw', category: 'sync' },
  { name: 'push', label: 'Push', description: 'Push to git remote', command: 'skillshare push', icon: 'Upload', category: 'sync' },
  { name: 'pull', label: 'Pull', description: 'Pull from git remote and sync', command: 'skillshare pull', icon: 'Download', category: 'sync' },
  { name: 'collect', label: 'Collect', description: 'Collect skills from target to source', command: 'skillshare collect', icon: 'RotateCcw', category: 'sync' },
  { name: 'diff', label: 'Diff', description: 'Show differences between source and targets', command: 'skillshare diff', icon: 'GitCompare', category: 'sync' },
  { name: 'status', label: 'Status', description: 'Show sync state', command: 'skillshare status', icon: 'Activity', category: 'sync' },

  // Skill Management
  { name: 'new', label: 'New', description: 'Create a new skill', command: 'skillshare new', icon: 'FilePlus', category: 'skill-mgmt' },
  { name: 'check', label: 'Check', description: 'Check for available updates', command: 'skillshare check', icon: 'Eye', category: 'skill-mgmt' },
  { name: 'update', label: 'Update', description: 'Update a skill or tracked repo', command: 'skillshare update', icon: 'RefreshCw', category: 'skill-mgmt' },
  { name: 'upgrade', label: 'Upgrade', description: 'Upgrade CLI or built-in skill', command: 'skillshare upgrade', icon: 'CircleArrowUp', category: 'skill-mgmt' },

  // Backup & Restore
  { name: 'backup', label: 'Backup', description: 'Create backup of targets', command: 'skillshare backup', icon: 'Archive', category: 'backup' },
  { name: 'restore', label: 'Restore', description: 'Restore targets from backup', command: 'skillshare restore', icon: 'RotateCcw', category: 'backup' },
  { name: 'trash-list', label: 'Trash List', description: 'List uninstalled skills in trash', command: 'skillshare trash list', icon: 'Trash2', category: 'backup' },
  { name: 'trash-restore', label: 'Trash Restore', description: 'Restore skill from trash', command: 'skillshare trash restore', icon: 'RotateCcw', category: 'backup' },

  // Security & Diagnostics
  { name: 'audit', label: 'Audit', description: 'Scan skills for security threats', command: 'skillshare audit', icon: 'ShieldCheck', category: 'security' },
  { name: 'log', label: 'Log', description: 'View operations and audit logs', command: 'skillshare log', icon: 'History', category: 'security' },
  { name: 'doctor', label: 'Doctor', description: 'Diagnose issues', command: 'skillshare doctor', icon: 'Stethoscope', category: 'security' },

  // Extras
  { name: 'extras-init', label: 'Extras Init', description: 'Initialize extras for a skill', command: 'skillshare extras init', icon: 'FolderPlus', category: 'extras' },
  { name: 'extras-list', label: 'Extras List', description: 'List extras resources', command: 'skillshare extras list', icon: 'List', category: 'extras' },
  { name: 'extras-remove', label: 'Extras Remove', description: 'Remove extras resource', command: 'skillshare extras remove', icon: 'Minus', category: 'extras' },
  { name: 'extras-collect', label: 'Extras Collect', description: 'Collect extras from targets', command: 'skillshare extras collect', icon: 'RotateCcw', category: 'extras' },

  // Other
  { name: 'target-list', label: 'Targets', description: 'List sync targets', command: 'skillshare target list', icon: 'Target', category: 'other' },
  { name: 'hub-list', label: 'Hub List', description: 'List skill hub sources', command: 'skillshare hub list', icon: 'Globe', category: 'other' },
  { name: 'hub-add', label: 'Hub Add', description: 'Add skill hub source', command: 'skillshare hub add', icon: 'Plus', category: 'other' },
  { name: 'config', label: 'Config', description: 'Manage configuration', command: 'skillshare config', icon: 'Settings', category: 'other' },
  { name: 'tui', label: 'TUI', description: 'Toggle interactive TUI mode', command: 'skillshare tui', icon: 'Terminal', category: 'other' },
  { name: 'ui', label: 'UI', description: 'Launch web dashboard', command: 'skillshare ui', icon: 'MonitorPlay', category: 'other' },
  { name: 'version', label: 'Version', description: 'Show CLI version', command: 'skillshare version', icon: 'Info', category: 'other' },
];

export const quickActionNames = [
  'sync', 'audit', 'status', 'doctor', 'target-list', 'list',
  'push', 'pull', 'diff', 'check',
];

export const quickActionCommands = skillshareCommands.filter((cmd) =>
  quickActionNames.includes(cmd.name)
);
