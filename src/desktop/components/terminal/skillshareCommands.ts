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
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export interface SkillshareCommand {
  name: string;
  label: string;
  description: string;
  command: string;
  icon: string;
}

export const commandIconMap: Record<string, LucideIcon> = {
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
};

export const skillshareCommands: SkillshareCommand[] = [
  {
    name: 'sync',
    label: 'Sync',
    description: 'Sync dotfiles with remote',
    command: 'skillshare sync',
    icon: 'RefreshCw',
  },
  {
    name: 'audit',
    label: 'Audit',
    description: 'Audit security of dotfiles',
    command: 'skillshare audit',
    icon: 'ShieldCheck',
  },
  {
    name: 'status',
    label: 'Status',
    description: 'Show sync status',
    command: 'skillshare status',
    icon: 'Activity',
  },
  {
    name: 'doctor',
    label: 'Doctor',
    description: 'Diagnose common issues',
    command: 'skillshare doctor',
    icon: 'Stethoscope',
  },
  {
    name: 'target-list',
    label: 'Targets',
    description: 'List sync targets',
    command: 'skillshare target list',
    icon: 'Target',
  },
  {
    name: 'list',
    label: 'List',
    description: 'List managed files',
    command: 'skillshare list',
    icon: 'List',
  },
  {
    name: 'init',
    label: 'Init',
    description: 'Initialize a new project',
    command: 'skillshare init',
    icon: 'FolderPlus',
  },
  {
    name: 'add',
    label: 'Add',
    description: 'Add files to tracking',
    command: 'skillshare add',
    icon: 'Plus',
  },
  {
    name: 'remove',
    label: 'Remove',
    description: 'Remove files from tracking',
    command: 'skillshare remove',
    icon: 'Minus',
  },
  {
    name: 'diff',
    label: 'Diff',
    description: 'Show file differences',
    command: 'skillshare diff',
    icon: 'GitCompare',
  },
  {
    name: 'log',
    label: 'Log',
    description: 'Show sync history',
    command: 'skillshare log',
    icon: 'History',
  },
  {
    name: 'config',
    label: 'Config',
    description: 'Manage configuration',
    command: 'skillshare config',
    icon: 'Settings',
  },
];

export const quickActionCommands = skillshareCommands.filter((cmd) =>
  ['sync', 'audit', 'status', 'doctor', 'target-list', 'list'].includes(cmd.name)
);
