/**
 * Settings Page Component
 * Discord-style two-column layout with search in sidebar
 * Desktop-optimized with focus management and keyboard navigation
 */

import React, { useState, useCallback, useEffect, useRef, Suspense, lazy } from 'react';
import {
  X,
  Loader2,
  Search,
  HardDrive,
  Palette,
  Keyboard,
  ArrowLeftRight,
  Sun,
  Moon,
  Bell,
  Info,
} from 'lucide-react';
import { McpIcon } from '../ui/McpIcon';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { cn } from '../../lib/utils';
import type { SettingsSection } from '../../types/settings';
import { SETTINGS_SEARCH_INDEX, type SettingsCategory } from '../../types/settings-search';
import { useTheme } from '../../contexts/ThemeContext';
import { useSettings } from '../../contexts/SettingsContext';
import { Toggle } from '../ui/Toggle';
import { Button } from '../ui/Button';

// Lazy load panel components
const StorageSettingsPanel = lazy(() =>
  import('./panels/StorageSettingsPanel').then((m) => ({ default: m.StorageSettingsPanel }))
);
const AppearanceSettingsPanel = lazy(() =>
  import('./panels/AppearanceSettingsPanel').then((m) => ({ default: m.AppearanceSettingsPanel }))
);
const ShortcutsSettingsPanel = lazy(() =>
  import('./panels/ShortcutsSettingsPanel').then((m) => ({ default: m.ShortcutsSettingsPanel }))
);
const McpSettingsFullPanel = lazy(() =>
  import('./panels/McpSettingsFullPanel').then((m) => ({ default: m.McpSettingsFullPanel }))
);
const NotificationSettingsPanel = lazy(() =>
  import('./panels/NotificationSettingsPanel').then((m) => ({
    default: m.NotificationSettingsPanel,
  }))
);
const DataSettingsPanel = lazy(() =>
  import('./panels/DataSettingsPanel').then((m) => ({ default: m.DataSettingsPanel }))
);
const AboutSettingsPanel = lazy(() =>
  import('./panels/AboutSettingsPanel').then((m) => ({ default: m.AboutSettingsPanel }))
);

interface SettingsPageProps {
  isOpen: boolean;
  onClose: () => void;
  initialSection?: SettingsSection;
  onImportComplete?: () => void;
}

// Icon mapping
const SECTION_ICONS: Record<SettingsSection, React.ElementType> = {
  storage: HardDrive,
  mcp: McpIcon,
  appearance: Palette,
  notifications: Bell,
  shortcuts: Keyboard,
  data: ArrowLeftRight,
  about: Info,
};

// Panel components mapping
const SECTION_PANELS: Record<
  SettingsSection,
  React.LazyExoticComponent<React.ComponentType<{ onExport?: () => void; onImport?: () => void }>>
> = {
  storage: StorageSettingsPanel,
  mcp: McpSettingsFullPanel,
  appearance: AppearanceSettingsPanel,
  notifications: NotificationSettingsPanel,
  shortcuts: ShortcutsSettingsPanel,
  data: DataSettingsPanel,
  about: AboutSettingsPanel,
};

// Sidebar categories structure (Discord-style grouping)
const SIDEBAR_CATEGORIES: { id: SettingsCategory; label: string; sections: SettingsSection[] }[] = [
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

// Get section info
function getSectionInfo(sectionId: SettingsSection) {
  return SETTINGS_SEARCH_INDEX.find((item) => item.id === sectionId);
}

// Simple fuzzy search
function searchSections(query: string): SettingsSection[] {
  if (!query.trim()) return [];

  const normalizedQuery = query.toLowerCase().trim();
  const results: { section: SettingsSection; score: number }[] = [];

  for (const item of SETTINGS_SEARCH_INDEX) {
    let score = 0;

    if (item.title.toLowerCase().includes(normalizedQuery)) {
      score = 100;
    } else if (item.description.toLowerCase().includes(normalizedQuery)) {
      score = 50;
    } else if (item.keywords.some((k) => k.toLowerCase().includes(normalizedQuery))) {
      score = 30;
    }

    if (score > 0) {
      results.push({ section: item.id, score });
    }
  }

  return results.sort((a, b) => b.score - a.score).map((r) => r.section);
}

// Loading fallback
const PanelLoadingFallback: React.FC = () => (
  <div className="flex items-center justify-center py-12">
    <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
  </div>
);

export const SettingsPage: React.FC<SettingsPageProps> = ({
  isOpen,
  onClose,
  initialSection = 'storage',
  onImportComplete,
}) => {
  const [activeSection, setActiveSection] = useState<SettingsSection>(initialSection);
  const [searchQuery, setSearchQuery] = useState('');
  const [isClosing, setIsClosing] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  const { theme, setTheme } = useTheme();
  const { pathDisplayFormat, setPathDisplayFormat } = useSettings();

  void onImportComplete; // kept for future use

  // Search results
  const searchResults = searchQuery ? searchSections(searchQuery) : [];
  const hasSearchQuery = searchQuery.trim().length > 0;

  // Detect fullscreen mode
  useEffect(() => {
    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    const checkFullscreen = async () => {
      const fullscreen = await appWindow.isFullscreen();
      setIsFullscreen(fullscreen);
    };

    const setupListener = async () => {
      checkFullscreen();
      unlisten = await appWindow.onResized(() => {
        checkFullscreen();
      });
    };

    setupListener();
    return () => {
      unlisten?.();
    };
  }, []);

  // Update active section when opening with a specific section
  useEffect(() => {
    if (isOpen && initialSection) {
      setActiveSection(initialSection);
    }
  }, [isOpen, initialSection]);

  // Close with animation
  const handleClose = useCallback(() => {
    setIsClosing(true);
    setTimeout(() => {
      setIsClosing(false);
      onClose();
    }, 150);
  }, [onClose]);

  // Focus management
  useEffect(() => {
    if (isOpen) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 100);
    } else if (previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
  }, [isOpen]);

  // Handle Escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (hasSearchQuery) {
          setSearchQuery('');
        } else {
          handleClose();
        }
      }
      // Cmd+F to focus search
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, hasSearchQuery, handleClose]);

  // Prevent body scroll
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen]);

  // Reset search when closing
  useEffect(() => {
    if (!isOpen) {
      setSearchQuery('');
    }
  }, [isOpen]);

  if (!isOpen && !isClosing) return null;

  const ActivePanelComponent = SECTION_PANELS[activeSection];

  return (
    <div
      className={cn(
        'fixed inset-0 z-50',
        'bg-background',
        'transition-opacity duration-150',
        'flex flex-col',
        isClosing ? 'opacity-0' : 'opacity-100 animate-in fade-in-0 duration-200'
      )}
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      {/* Header - Drag region with close button */}
      <header
        data-tauri-drag-region
        className={cn(
          'h-12 shrink-0 flex items-center justify-end pr-3',
          isFullscreen ? '' : 'pl-[72px]'
        )}
      >
        <Button
          ref={closeButtonRef}
          variant="ghost"
          size="icon"
          onClick={handleClose}
          aria-label="Close settings (Escape)"
        >
          <X className="w-4 h-4" />
        </Button>
      </header>

      {/* Main Content - Two Column Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <aside className="w-60 shrink-0 border-r border-border flex flex-col bg-card/50">
          {/* Sidebar Header */}
          <div className="px-4 py-3">
            <h1 id="settings-title" className="text-lg font-semibold text-foreground">
              Settings
            </h1>
          </div>

          {/* Search */}
          <div className="px-3 pb-3">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search"
                className={cn(
                  'w-full pl-9 pr-3 py-2 rounded-md',
                  'bg-muted/50 border border-transparent',
                  'text-sm placeholder:text-muted-foreground',
                  'focus:outline-none focus:bg-background focus:border-border',
                  'transition-colors'
                )}
              />
            </div>
          </div>

          {/* Quick Settings - Gradient Card */}
          <div className="px-3 pb-3">
            <div
              className={cn(
                'p-3 rounded-lg space-y-3',
                'bg-gradient-to-r from-primary/5 to-purple-500/5',
                'border border-primary/10'
              )}
            >
              {/* Theme Toggle */}
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-muted-foreground">Theme</span>
                <div className="flex gap-0.5 p-0.5 bg-muted/80 rounded-md">
                  <Button
                    type="button"
                    variant="ghost"
                    onClick={() => setTheme('light')}
                    className={cn(
                      'flex items-center gap-1.5 px-2 py-1 rounded text-xs h-auto',
                      theme === 'light'
                        ? 'bg-background text-foreground shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    )}
                  >
                    <Sun className="w-3 h-3" />
                    <span>Light</span>
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    onClick={() => setTheme('dark')}
                    className={cn(
                      'flex items-center gap-1.5 px-2 py-1 rounded text-xs h-auto',
                      theme === 'dark'
                        ? 'bg-background text-foreground shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    )}
                  >
                    <Moon className="w-3 h-3" />
                    <span>Dark</span>
                  </Button>
                </div>
              </div>

              {/* Compact Paths Toggle */}
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-muted-foreground">Compact Paths</span>
                <Toggle
                  checked={pathDisplayFormat === 'short'}
                  onChange={() =>
                    setPathDisplayFormat(pathDisplayFormat === 'short' ? 'full' : 'short')
                  }
                  size="sm"
                  aria-label="Toggle compact paths"
                />
              </div>
            </div>
          </div>

          {/* Navigation */}
          <nav className="flex-1 overflow-y-auto px-2 pb-4">
            {hasSearchQuery ? (
              // Search Results
              <div className="space-y-1">
                <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                  Search Results
                </div>
                {searchResults.length === 0 ? (
                  <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                    No results found
                  </div>
                ) : (
                  searchResults.map((sectionId) => {
                    const info = getSectionInfo(sectionId);
                    const Icon = SECTION_ICONS[sectionId];
                    if (!info) return null;

                    return (
                      <Button
                        key={sectionId}
                        variant="ghost"
                        onClick={() => {
                          setActiveSection(sectionId);
                          setSearchQuery('');
                        }}
                        className={cn(
                          'w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm h-auto justify-start',
                          activeSection === sectionId
                            ? 'bg-accent text-foreground'
                            : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
                        )}
                      >
                        <Icon className="w-4 h-4" />
                        <span>{info.title}</span>
                      </Button>
                    );
                  })
                )}
              </div>
            ) : (
              // Category Groups
              SIDEBAR_CATEGORIES.map((category) => (
                <div key={category.id} className="mb-4">
                  <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {category.label}
                  </div>
                  <div className="space-y-0.5">
                    {category.sections.map((sectionId) => {
                      const info = getSectionInfo(sectionId);
                      const Icon = SECTION_ICONS[sectionId];
                      if (!info) return null;

                      return (
                        <Button
                          key={sectionId}
                          variant="ghost"
                          onClick={() => setActiveSection(sectionId)}
                          className={cn(
                            'w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm h-auto justify-start',
                            activeSection === sectionId
                              ? 'bg-accent text-foreground'
                              : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
                          )}
                        >
                          <Icon className="w-4 h-4" />
                          <span>{info.title}</span>
                        </Button>
                      );
                    })}
                  </div>
                </div>
              ))
            )}
          </nav>
        </aside>

        {/* Content Area */}
        <main className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 max-w-3xl w-full mx-auto p-6 flex flex-col min-h-0">
            <Suspense fallback={<PanelLoadingFallback />}>
              <ActivePanelComponent />
            </Suspense>
          </div>
        </main>
      </div>
    </div>
  );
};
