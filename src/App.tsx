import { useState, useCallback, useMemo, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { WorkflowPage } from './components/workflow/WorkflowPage';
import { SpecList } from './components/spec-editor/SpecList';
import { SpecEditor } from './components/spec-editor/SpecEditor';
import { useShortcutsContext } from './contexts/ShortcutsContext';
import { SettingsButton } from './components/settings/SettingsButton';
import { SettingsPage } from './components/settings/SettingsPage';
import type { SettingsSection } from './types/settings';
import { useKeyboardShortcuts, type KeyboardShortcut } from './hooks/useKeyboardShortcuts';
import { ShortcutToast } from './components/ui/KeyboardShortcutsHint';
import {
  KeyboardShortcutsDialog,
  KeyboardShortcutsFloatingButton,
} from './components/ui/KeyboardShortcutsDialog';
import { useUpdater } from './hooks/useUpdater';
import { UpdateDialog } from './components/ui/UpdateDialog';
import { useMcpStatus } from './hooks/useMcpStatus';
import {
  NotificationButton,
  BackgroundTasksButton,
  McpStatusButton,
} from './components/status-bar';
import { ActionConfirmationDialog } from './components/settings/mcp';
import { FirstRunWizard } from './components/onboarding/FirstRunWizard';
import { cn } from './lib/utils';

type AppTab = 'specs' | 'workflows';

// TODO: projectDir should come from a project context / settings.
// Using the current working directory as a reasonable default for now.
const PROJECT_DIR = '.';

const DEFAULT_SHORTCUT_KEYS: Record<string, string> = {
  refresh: 'cmd+r',
  new: 'cmd+n',
  save: 'cmd+s',
  search: 'cmd+f',
  export: 'cmd+e',
  import: 'cmd+i',
  'tab-workflows': 'cmd+1',
  help: 'cmd+/',
  settings: 'cmd+,',
};

function App() {
  // Update dialog state
  const {
    dialogOpen: updateDialogOpen,
    setDialogOpen: setUpdateDialogOpen,
    state: updateState,
    currentVersion,
    newVersion,
    releaseNotes,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    error: updateError,
    startUpdate,
    dismissUpdate,
    restartApp,
    retryUpdate,
  } = useUpdater();

  // First-run wizard state
  const [showWizard, setShowWizard] = useState<boolean | null>(null);

  useEffect(() => {
    invoke<boolean>('check_specforge_exists', { projectDir: PROJECT_DIR }).then(
      (exists) => setShowWizard(!exists),
      () => setShowWizard(false) // On error, skip wizard
    );
  }, []);

  const handleWizardComplete = useCallback(() => {
    setShowWizard(false);
    setDataVersion((prev) => prev + 1);
  }, []);

  const [activeTab, setActiveTab] = useState<AppTab>('specs');
  const [selectedSpecId, setSelectedSpecId] = useState<string | null>(null);
  const [settingsPageOpen, setSettingsPageOpen] = useState(false);
  const [settingsInitialSection, setSettingsInitialSection] = useState<SettingsSection>('storage');
  const [dataVersion, setDataVersion] = useState(0);
  const [shortcutToast, setShortcutToast] = useState<{ message: string; key: string } | null>(null);
  const [shortcutsDialogOpen, setShortcutsDialogOpen] = useState(false);
  const mcpStatus = useMcpStatus();

  const showShortcutToast = useCallback((message: string, key: string) => {
    setShortcutToast({ message, key });
  }, []);

  const handleImportComplete = useCallback(() => {
    setDataVersion((prev) => prev + 1);
  }, []);

  const openSettings = useCallback((section?: SettingsSection) => {
    if (section) {
      setSettingsInitialSection(section);
    }
    setSettingsPageOpen(true);
  }, []);

  const { getEffectiveKey, isShortcutEnabled } = useShortcutsContext();

  const shortcuts: KeyboardShortcut[] = useMemo(
    () => [
      {
        id: 'refresh',
        key: getEffectiveKey('refresh', DEFAULT_SHORTCUT_KEYS['refresh']),
        description: 'Reload workflows',
        category: 'General',
        enabled: isShortcutEnabled('refresh'),
        action: () => {
          setDataVersion((prev) => prev + 1);
          showShortcutToast(
            'Reloaded',
            getEffectiveKey('refresh', DEFAULT_SHORTCUT_KEYS['refresh'])
          );
        },
      },
      {
        id: 'new',
        key: getEffectiveKey('new', DEFAULT_SHORTCUT_KEYS['new']),
        description: 'New workflow',
        category: 'General',
        enabled: isShortcutEnabled('new'),
        action: () => {
          const effectiveKey = getEffectiveKey('new', DEFAULT_SHORTCUT_KEYS['new']);
          window.dispatchEvent(new CustomEvent('shortcut-new-workflow'));
          showShortcutToast('New Workflow', effectiveKey);
        },
      },
      {
        id: 'save',
        key: getEffectiveKey('save', DEFAULT_SHORTCUT_KEYS['save']),
        description: 'Save current workflow',
        category: 'General',
        enabled: isShortcutEnabled('save'),
        action: () => {
          window.dispatchEvent(new CustomEvent('shortcut-save-workflow'));
          showShortcutToast('Saved', getEffectiveKey('save', DEFAULT_SHORTCUT_KEYS['save']));
        },
      },
      {
        id: 'search',
        key: getEffectiveKey('search', DEFAULT_SHORTCUT_KEYS['search']),
        description: 'Focus search',
        category: 'General',
        enabled: isShortcutEnabled('search'),
        action: () => {
          window.dispatchEvent(new CustomEvent('shortcut-focus-search'));
        },
      },
      {
        id: 'export',
        key: getEffectiveKey('export', DEFAULT_SHORTCUT_KEYS['export']),
        description: 'Export data',
        category: 'Data',
        enabled: isShortcutEnabled('export'),
        action: () => {
          openSettings('data');
          setTimeout(() => {
            window.dispatchEvent(new CustomEvent('settings-open-export'));
          }, 100);
        },
      },
      {
        id: 'import',
        key: getEffectiveKey('import', DEFAULT_SHORTCUT_KEYS['import']),
        description: 'Import data',
        category: 'Data',
        enabled: isShortcutEnabled('import'),
        action: () => {
          openSettings('data');
          setTimeout(() => {
            window.dispatchEvent(new CustomEvent('settings-open-import'));
          }, 100);
        },
      },
      {
        id: 'tab-workflows',
        key: getEffectiveKey('tab-workflows', DEFAULT_SHORTCUT_KEYS['tab-workflows']),
        description: 'Switch to Workflows tab',
        category: 'Navigation',
        enabled: isShortcutEnabled('tab-workflows'),
        action: () => {
          setActiveTab('workflows');
          showShortcutToast(
            'Workflows',
            getEffectiveKey('tab-workflows', DEFAULT_SHORTCUT_KEYS['tab-workflows'])
          );
        },
      },
      {
        id: 'help',
        key: getEffectiveKey('help', DEFAULT_SHORTCUT_KEYS['help']),
        description: 'Show keyboard shortcuts',
        category: 'Help',
        enabled: isShortcutEnabled('help'),
        action: () => {
          setShortcutsDialogOpen(true);
        },
      },
      {
        id: 'settings',
        key: getEffectiveKey('settings', DEFAULT_SHORTCUT_KEYS['settings']),
        description: 'Open Settings',
        category: 'General',
        enabled: isShortcutEnabled('settings'),
        action: () => {
          openSettings();
          showShortcutToast(
            'Settings',
            getEffectiveKey('settings', DEFAULT_SHORTCUT_KEYS['settings'])
          );
        },
      },
    ],
    [showShortcutToast, getEffectiveKey, isShortcutEnabled, openSettings, setActiveTab]
  );

  const displayShortcuts: KeyboardShortcut[] = useMemo(
    () => [
      {
        id: 'refresh',
        key: DEFAULT_SHORTCUT_KEYS['refresh'],
        description: 'Reload workflows',
        category: 'General',
        action: () => {},
      },
      {
        id: 'new',
        key: DEFAULT_SHORTCUT_KEYS['new'],
        description: 'New workflow',
        category: 'General',
        action: () => {},
      },
      {
        id: 'save',
        key: DEFAULT_SHORTCUT_KEYS['save'],
        description: 'Save current workflow',
        category: 'General',
        action: () => {},
      },
      {
        id: 'search',
        key: DEFAULT_SHORTCUT_KEYS['search'],
        description: 'Focus search',
        category: 'General',
        action: () => {},
      },
      {
        id: 'settings',
        key: DEFAULT_SHORTCUT_KEYS['settings'],
        description: 'Open Settings',
        category: 'General',
        action: () => {},
      },
      {
        id: 'export',
        key: DEFAULT_SHORTCUT_KEYS['export'],
        description: 'Export data',
        category: 'Data',
        action: () => {},
      },
      {
        id: 'import',
        key: DEFAULT_SHORTCUT_KEYS['import'],
        description: 'Import data',
        category: 'Data',
        action: () => {},
      },
      {
        id: 'tab-workflows',
        key: DEFAULT_SHORTCUT_KEYS['tab-workflows'],
        description: 'Switch to Workflows tab',
        category: 'Navigation',
        action: () => {},
      },
      {
        id: 'help',
        key: DEFAULT_SHORTCUT_KEYS['help'],
        description: 'Show keyboard shortcuts',
        category: 'Help',
        action: () => {},
      },
    ],
    []
  );

  useKeyboardShortcuts(shortcuts);

  // Show wizard if .specforge/ doesn't exist (null = still checking)
  if (showWizard === true) {
    return <FirstRunWizard projectDir={PROJECT_DIR} onComplete={handleWizardComplete} />;
  }

  // Still checking — render nothing to avoid flash
  if (showWizard === null) {
    return <div className="h-screen bg-background" />;
  }

  return (
    <div className="h-screen flex flex-col bg-background rounded-lg overflow-hidden select-none">
      <header
        data-tauri-drag-region
        className="flex items-center border-b border-border bg-card h-12 flex-shrink-0"
      >
        {/* Left: Space for macOS traffic lights */}
        <div data-tauri-drag-region className="w-20 h-full" />

        {/* Tab navigation */}
        <nav className="flex items-center gap-1 h-full">
          <button
            type="button"
            onClick={() => setActiveTab('specs')}
            className={cn(
              'px-3 h-full text-sm font-medium transition-colors duration-150 border-b-2',
              activeTab === 'specs'
                ? 'text-foreground border-blue-500'
                : 'text-muted-foreground border-transparent hover:text-foreground'
            )}
          >
            Specs
          </button>
          <button
            type="button"
            onClick={() => setActiveTab('workflows')}
            className={cn(
              'px-3 h-full text-sm font-medium transition-colors duration-150 border-b-2',
              activeTab === 'workflows'
                ? 'text-foreground border-blue-500'
                : 'text-muted-foreground border-transparent hover:text-foreground'
            )}
          >
            Workflows
          </button>
        </nav>

        {/* Center: Draggable region */}
        <div data-tauri-drag-region className="flex-1 h-full" />
        <div className="flex items-center gap-1 px-2">
          <BackgroundTasksButton />
          <McpStatusButton
            config={mcpStatus.config}
            isLoading={mcpStatus.isLoading}
            onOpenSettings={() => openSettings('mcp')}
          />
          <NotificationButton />
          <div className="w-px h-5 bg-border mx-1" />
          <SettingsButton onClick={() => openSettings()} />
        </div>
      </header>

      <main className="flex-1 flex flex-col overflow-hidden bg-background">
        <div className="flex-1 overflow-hidden">
          {activeTab === 'specs' ? (
            selectedSpecId ? (
              <SpecEditor
                specId={selectedSpecId}
                projectDir={PROJECT_DIR}
                onBack={() => setSelectedSpecId(null)}
              />
            ) : (
              <SpecList
                projectDir={PROJECT_DIR}
                onSelectSpec={(specId) => setSelectedSpecId(specId)}
              />
            )
          ) : (
            <WorkflowPage dataVersion={dataVersion} />
          )}
        </div>
      </main>

      <SettingsPage
        isOpen={settingsPageOpen}
        onClose={() => setSettingsPageOpen(false)}
        initialSection={settingsInitialSection}
        onImportComplete={handleImportComplete}
      />

      <KeyboardShortcutsFloatingButton
        onClick={() => setShortcutsDialogOpen(true)}
        position="bottom-right"
        bottomOffset={250}
      />

      <KeyboardShortcutsDialog
        open={shortcutsDialogOpen}
        onOpenChange={setShortcutsDialogOpen}
        shortcuts={displayShortcuts}
        onCustomize={() => openSettings('shortcuts')}
      />

      <ShortcutToast
        message={shortcutToast?.message || ''}
        shortcutKey={shortcutToast?.key || ''}
        visible={!!shortcutToast}
        onHide={() => setShortcutToast(null)}
      />

      {/* Update Dialog */}
      {(updateState === 'available' ||
        updateState === 'downloading' ||
        updateState === 'installing' ||
        updateState === 'complete' ||
        updateState === 'error') && (
        <UpdateDialog
          open={updateDialogOpen}
          onOpenChange={setUpdateDialogOpen}
          state={updateState}
          currentVersion={currentVersion}
          newVersion={newVersion || ''}
          releaseNotes={releaseNotes}
          downloadProgress={downloadProgress}
          downloadedBytes={downloadedBytes}
          totalBytes={totalBytes}
          errorMessage={updateError}
          onUpdate={startUpdate}
          onLater={dismissUpdate}
          onRestart={restartApp}
          onRetry={retryUpdate}
        />
      )}

      {/* MCP Action Confirmation Dialog - floating approval UI */}
      <ActionConfirmationDialog position="bottom-right" />
    </div>
  );
}

export default App;
