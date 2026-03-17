/**
 * Settings Search Types
 * Types for the settings search functionality
 */

import type { SettingsSection } from './settings';

export type SettingsCategory = 'project' | 'preferences' | 'mcp' | 'data' | 'about';

export interface SettingsSearchIndexItem {
  id: SettingsSection;
  category: SettingsCategory;
  title: string;
  description: string;
  keywords: string[];
}

export interface SettingsSearchResult {
  item: SettingsSearchIndexItem;
  matchedField: 'title' | 'description' | 'keyword';
  matchedText: string;
  score: number;
}

export interface SettingsCategoryInfo {
  id: SettingsCategory | 'all';
  label: string;
  sections: SettingsSection[];
}

/**
 * Settings search index - defines all searchable settings
 */
export const SETTINGS_SEARCH_INDEX: SettingsSearchIndexItem[] = [
  {
    id: 'storage',
    category: 'project',
    title: 'Storage',
    description: 'Configure data storage locations',
    keywords: ['database', 'path', 'folder', 'location', 'sqlite', 'data', 'directory'],
  },
  {
    id: 'mcp',
    category: 'mcp',
    title: 'MCP Integration',
    description: 'Configure Model Context Protocol server and tools',
    keywords: ['mcp', 'model', 'context', 'protocol', 'server', 'tools', 'integration'],
  },
  {
    id: 'appearance',
    category: 'preferences',
    title: 'Appearance',
    description: 'Customize theme and visual settings',
    keywords: ['theme', 'dark', 'light', 'mode', 'color', 'style', 'path', 'compact'],
  },
  {
    id: 'notifications',
    category: 'preferences',
    title: 'Notifications',
    description: 'Configure desktop notification preferences',
    keywords: ['notification', 'alert', 'sound', 'dnd', 'do not disturb', 'bell', 'desktop'],
  },
  {
    id: 'shortcuts',
    category: 'preferences',
    title: 'Keyboard Shortcuts',
    description: 'View and customize keyboard shortcuts',
    keywords: ['keyboard', 'shortcut', 'hotkey', 'keybinding', 'key', 'binding'],
  },
  {
    id: 'data',
    category: 'data',
    title: 'Import / Export',
    description: 'Backup and restore your data',
    keywords: ['import', 'export', 'backup', 'restore', 'json', 'data'],
  },
  {
    id: 'about',
    category: 'about',
    title: 'About',
    description: 'View app version and check for updates',
    keywords: ['version', 'update', 'about', 'info', 'changelog', 'release'],
  },
];

/**
 * Category definitions with their associated sections
 */
export const SETTINGS_CATEGORIES: SettingsCategoryInfo[] = [
  {
    id: 'all',
    label: 'All',
    sections: ['storage', 'mcp', 'appearance', 'notifications', 'shortcuts', 'data', 'about'],
  },
  {
    id: 'project',
    label: 'Project',
    sections: ['storage'],
  },
  {
    id: 'mcp',
    label: 'MCP',
    sections: ['mcp'],
  },
  {
    id: 'preferences',
    label: 'Preferences',
    sections: ['appearance', 'notifications', 'shortcuts'],
  },
  {
    id: 'data',
    label: 'Data',
    sections: ['data'],
  },
  {
    id: 'about',
    label: 'About',
    sections: ['about'],
  },
];

/**
 * Get category info for a section
 */
export function getCategoryForSection(sectionId: SettingsSection): SettingsCategory {
  const item = SETTINGS_SEARCH_INDEX.find((i) => i.id === sectionId);
  return item?.category ?? 'project';
}

/**
 * Get sections for a category
 */
export function getSectionsForCategory(categoryId: SettingsCategory | 'all'): SettingsSection[] {
  const category = SETTINGS_CATEGORIES.find((c) => c.id === categoryId);
  return category?.sections ?? [];
}
