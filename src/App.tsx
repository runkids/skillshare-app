import { useState, useCallback, useMemo } from 'react';
import { WorkflowPage } from './components/workflow/WorkflowPage';
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

type AppTab = 'workflows';

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

  const [_activeTab] = useState<AppTab>('workflows');
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
    [showShortcutToast, getEffectiveKey, isShortcutEnabled, openSettings]
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

  return (
    <div className="h-screen flex flex-col bg-background rounded-lg overflow-hidden select-none">
      <header
        data-tauri-drag-region
        className="flex items-center border-b border-border bg-card h-12 flex-shrink-0"
      >
        {/* Left: Space for macOS traffic lights */}
        <div data-tauri-drag-region className="w-20 h-full" />
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
          <WorkflowPage dataVersion={dataVersion} />
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
