/**
 * Settings Page Types
 */

export type SettingsSection =
  | 'storage'
  | 'appearance'
  | 'notifications'
  | 'shortcuts'
  | 'mcp'
  | 'data'
  | 'about';

export interface SettingsSidebarSection {
  id: string;
  label: string;
  items: SettingsSidebarItem[];
}

export interface SettingsSidebarItem {
  id: SettingsSection;
  label: string;
  icon: string;
  badge?: string;
}

export const SETTINGS_SECTIONS: SettingsSidebarSection[] = [
  {
    id: 'project',
    label: 'Project',
    items: [{ id: 'storage', label: 'Storage', icon: 'HardDrive' }],
  },
  {
    id: 'preferences',
    label: 'Preferences',
    items: [
      { id: 'appearance', label: 'Appearance', icon: 'Palette' },
      { id: 'notifications', label: 'Notifications', icon: 'Bell' },
      { id: 'shortcuts', label: 'Keyboard Shortcuts', icon: 'Keyboard' },
    ],
  },
  {
    id: 'mcp',
    label: 'MCP',
    items: [{ id: 'mcp', label: 'MCP Integration', icon: 'Server' }],
  },
  {
    id: 'data',
    label: 'Data',
    items: [{ id: 'data', label: 'Import / Export', icon: 'ArrowLeftRight' }],
  },
];
