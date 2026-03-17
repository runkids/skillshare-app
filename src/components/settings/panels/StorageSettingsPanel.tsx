/**
 * Storage Settings Panel
 * Display SQLite database storage location and information
 *
 * Features:
 * - Storage location with quick access
 * - Database information card
 * - WAL mode status indicator
 * - Export/Import guidance
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  HardDrive,
  FolderOpen,
  Database,
  CheckCircle2,
  Shield,
  Info,
  Copy,
  Check,
  Clock,
  Trash2,
  RefreshCw,
  Archive,
  Eye,
  EyeOff,
} from 'lucide-react';
import { Button } from '../../ui/Button';
import { settingsAPI } from '../../../lib/tauri-api';
import { SettingSection } from '../ui/SettingSection';
import { SettingInfoBox } from '../ui/SettingInfoBox';
import { Skeleton } from '../../ui/Skeleton';
import { cn } from '../../../lib/utils';
import { useSettings } from '../../../contexts/SettingsContext';
import type { StorePathInfo } from '../../../types/tauri';

// Stub types for removed snapshot/time-machine features
type SnapshotStorageStats = {
  totalSnapshots: number;
  totalSizeBytes: number;
  totalSizeHuman: string;
  projectCount: number;
};
type TimeMachineSettings = {
  autoWatchEnabled: boolean;
  retentionDays: number;
  maxSnapshotsPerProject: number;
  debounceMs: number;
  updatedAt?: string;
  [key: string]: unknown;
};
// Stub API for removed snapshot features
const snapshotAPI = {
  getStorageStats: async (): Promise<SnapshotStorageStats> => ({
    totalSnapshots: 0,
    totalSizeBytes: 0,
    totalSizeHuman: '0 B',
    projectCount: 0,
  }),
  pruneSnapshots: async (_keepDays?: number): Promise<number> => 0,
  cleanupOrphanedStorage: async (): Promise<number> => 0,
  getTimeMachineSettings: async (): Promise<TimeMachineSettings> => ({
    autoWatchEnabled: false,
    retentionDays: 30,
    maxSnapshotsPerProject: 10,
    debounceMs: 2000,
  }),
  updateTimeMachineSettings: async (_settings: TimeMachineSettings): Promise<void> => {},
};

export const StorageSettingsPanel: React.FC = () => {
  const { formatPath } = useSettings();
  const [storePathInfo, setStorePathInfo] = useState<StorePathInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [copied, setCopied] = useState(false);

  // Time Machine storage state
  const [snapshotStats, setSnapshotStats] = useState<SnapshotStorageStats | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(true);
  const [keepPerProject, setKeepPerProject] = useState(10);
  const [isPruning, setIsPruning] = useState(false);
  const [isCleaningUp, setIsCleaningUp] = useState(false);
  const [pruneResult, setPruneResult] = useState<{ count: number; type: string } | null>(null);

  // Time Machine auto-watch settings
  const [timeMachineSettings, setTimeMachineSettings] = useState<TimeMachineSettings | null>(null);
  const [isLoadingTMSettings, setIsLoadingTMSettings] = useState(true);
  const [isSavingTMSettings, setIsSavingTMSettings] = useState(false);

  // Formatted storage path for display (respects Compact Paths setting)
  const displayPath = useMemo(() => {
    if (!storePathInfo?.currentPath) return null;
    return formatPath(storePathInfo.currentPath);
  }, [storePathInfo?.currentPath, formatPath]);

  // Extract just the filename from the path
  const fileName = useMemo(() => {
    if (!storePathInfo?.currentPath) return null;
    const parts = storePathInfo.currentPath.split('/');
    return parts[parts.length - 1];
  }, [storePathInfo?.currentPath]);

  // Load store path info on mount
  useEffect(() => {
    const loadPathInfo = async () => {
      try {
        setIsLoading(true);
        const info = await settingsAPI.getStorePath();
        setStorePathInfo(info);
      } catch (error) {
        console.error('Failed to load store path info:', error);
      } finally {
        setIsLoading(false);
      }
    };
    loadPathInfo();
  }, []);

  // Load snapshot storage stats
  const loadSnapshotStats = useCallback(async () => {
    try {
      setIsLoadingStats(true);
      const stats = await snapshotAPI.getStorageStats();
      setSnapshotStats(stats);
    } catch (error) {
      console.error('Failed to load snapshot stats:', error);
    } finally {
      setIsLoadingStats(false);
    }
  }, []);

  useEffect(() => {
    loadSnapshotStats();
  }, [loadSnapshotStats]);

  // Load Time Machine settings
  const loadTimeMachineSettings = useCallback(async () => {
    try {
      setIsLoadingTMSettings(true);
      const settings = await snapshotAPI.getTimeMachineSettings();
      setTimeMachineSettings(settings);
    } catch (error) {
      console.error('Failed to load Time Machine settings:', error);
    } finally {
      setIsLoadingTMSettings(false);
    }
  }, []);

  useEffect(() => {
    loadTimeMachineSettings();
  }, [loadTimeMachineSettings]);

  // Handle toggle auto-watch
  const handleToggleAutoWatch = useCallback(async () => {
    if (!timeMachineSettings) return;
    try {
      setIsSavingTMSettings(true);
      const newSettings: TimeMachineSettings = {
        ...timeMachineSettings,
        autoWatchEnabled: !timeMachineSettings.autoWatchEnabled,
        updatedAt: new Date().toISOString(),
      };
      await snapshotAPI.updateTimeMachineSettings(newSettings);
      setTimeMachineSettings(newSettings);
    } catch (error) {
      console.error('Failed to update auto-watch setting:', error);
    } finally {
      setIsSavingTMSettings(false);
    }
  }, [timeMachineSettings]);

  // Handle update debounce
  const handleUpdateDebounce = useCallback(
    async (newDebounceMs: number) => {
      if (!timeMachineSettings) return;
      try {
        setIsSavingTMSettings(true);
        const newSettings: TimeMachineSettings = {
          ...timeMachineSettings,
          debounceMs: newDebounceMs,
          updatedAt: new Date().toISOString(),
        };
        await snapshotAPI.updateTimeMachineSettings(newSettings);
        setTimeMachineSettings(newSettings);
      } catch (error) {
        console.error('Failed to update debounce setting:', error);
      } finally {
        setIsSavingTMSettings(false);
      }
    },
    [timeMachineSettings]
  );

  // Handle prune snapshots
  const handlePruneSnapshots = useCallback(async () => {
    try {
      setIsPruning(true);
      setPruneResult(null);
      const deleted = await snapshotAPI.pruneSnapshots(keepPerProject);
      setPruneResult({ count: deleted, type: 'prune' });
      // Reload stats after pruning
      await loadSnapshotStats();
    } catch (error) {
      console.error('Failed to prune snapshots:', error);
    } finally {
      setIsPruning(false);
    }
  }, [keepPerProject, loadSnapshotStats]);

  // Handle cleanup orphaned storage
  const handleCleanupOrphaned = useCallback(async () => {
    try {
      setIsCleaningUp(true);
      setPruneResult(null);
      const deleted = await snapshotAPI.cleanupOrphanedStorage();
      setPruneResult({ count: deleted, type: 'cleanup' });
      // Reload stats after cleanup
      await loadSnapshotStats();
    } catch (error) {
      console.error('Failed to cleanup orphaned storage:', error);
    } finally {
      setIsCleaningUp(false);
    }
  }, [loadSnapshotStats]);

  // Handle open store location in file explorer
  const handleOpenLocation = useCallback(async () => {
    try {
      await settingsAPI.openStoreLocation();
    } catch (error) {
      console.error('Failed to open store location:', error);
    }
  }, []);

  // Handle copy path to clipboard
  const handleCopyPath = useCallback(async () => {
    if (!storePathInfo?.currentPath) return;
    try {
      await navigator.clipboard.writeText(storePathInfo.currentPath);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy path:', error);
    }
  }, [storePathInfo?.currentPath]);

  return (
    <div className="flex flex-col h-full">
      {/* Fixed Header */}
      <div className="flex-shrink-0 pb-4 border-b border-border bg-background">
        <h2 className="text-xl font-semibold text-foreground flex items-center">
          <HardDrive className="w-5 h-5 pr-1" />
          Storage
        </h2>
        <p className="text-sm text-muted-foreground mt-1">View where PackageFlow stores its data</p>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pt-4 space-y-6">
        {/* Storage Location Section */}
        <SettingSection
          title="Database Location"
          description="Your data is stored in a local SQLite database"
          icon={<Database className="w-4 h-4" />}
        >
          {isLoading ? (
            <StorageLocationSkeleton />
          ) : (
            <div className="space-y-3">
              {/* Path Display Card */}
              <div
                className={cn(
                  'group relative p-4 rounded-lg',
                  'bg-gradient-to-r from-blue-500/5 via-transparent to-transparent',
                  'border border-blue-500/20',
                  'transition-colors hover:border-blue-500/40'
                )}
              >
                <div className="flex items-start gap-3">
                  {/* Database Icon */}
                  <div
                    className={cn('flex-shrink-0 p-2.5 rounded-lg', 'bg-blue-500/10 text-blue-500')}
                  >
                    <Database className="w-5 h-5" />
                  </div>

                  {/* Path Info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-foreground">
                        {fileName || 'packageflow.db'}
                      </span>
                      <span
                        className={cn(
                          'inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium',
                          'bg-green-500/10 text-green-600 dark:text-green-400 border border-green-500/20'
                        )}
                      >
                        <CheckCircle2 className="w-3 h-3" />
                        WAL Mode
                      </span>
                    </div>
                    <code
                      className="block mt-1 text-xs text-muted-foreground font-mono truncate"
                      title={displayPath || undefined}
                    >
                      {displayPath || 'Not configured'}
                    </code>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1 flex-shrink-0">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={handleCopyPath}
                      className="h-8 w-8 text-muted-foreground hover:text-foreground"
                      title="Copy full path"
                    >
                      {copied ? (
                        <Check className="w-4 h-4 text-green-500" />
                      ) : (
                        <Copy className="w-4 h-4" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={handleOpenLocation}
                      className="h-8 w-8 text-muted-foreground hover:text-foreground"
                      title="Open in Finder"
                    >
                      <FolderOpen className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
              </div>

              {/* Open in Finder Button */}
              <Button variant="outline" onClick={handleOpenLocation} className="w-full">
                <FolderOpen className="w-4 h-4 pr-1" />
                Reveal in Finder
              </Button>
            </div>
          )}
        </SettingSection>

        {/* Database Features Section */}
        <SettingSection
          title="Database Features"
          description="SQLite provides reliable, high-performance local storage"
          icon={<Shield className="w-4 h-4" />}
        >
          <div className="grid gap-3">
            <FeatureCard
              icon={<CheckCircle2 className="w-4 h-4" />}
              title="Write-Ahead Logging (WAL)"
              description="Enables concurrent reads while writing, preventing data corruption"
              variant="success"
            />
            <FeatureCard
              icon={<Shield className="w-4 h-4" />}
              title="ACID Compliance"
              description="Atomic transactions ensure data integrity even during crashes"
              variant="info"
            />
            <FeatureCard
              icon={<Database className="w-4 h-4" />}
              title="Local Storage"
              description="All data stays on your device - no cloud sync or external servers"
              variant="default"
            />
          </div>
        </SettingSection>

        {/* Time Machine Storage Section */}
        <SettingSection
          title="Time Machine Storage"
          description="Manage execution snapshots and storage usage"
          icon={<Clock className="w-4 h-4" />}
        >
          {isLoadingStats || isLoadingTMSettings ? (
            <TimeMachineStorageSkeleton />
          ) : (
            <div className="space-y-4">
              {/* Auto-Watch Setting */}
              {timeMachineSettings && (
                <div
                  className={cn(
                    'p-4 rounded-lg',
                    'bg-gradient-to-r from-cyan-500/5 via-transparent to-transparent',
                    'border border-cyan-500/20'
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div
                        className={cn(
                          'p-2.5 rounded-lg',
                          timeMachineSettings.autoWatchEnabled
                            ? 'bg-cyan-500/10 text-cyan-500'
                            : 'bg-muted text-muted-foreground'
                        )}
                      >
                        {timeMachineSettings.autoWatchEnabled ? (
                          <Eye className="w-5 h-5" />
                        ) : (
                          <EyeOff className="w-5 h-5" />
                        )}
                      </div>
                      <div>
                        <div className="text-sm font-medium text-foreground">
                          Auto-Watch Lockfiles
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {timeMachineSettings.autoWatchEnabled
                            ? 'Automatically capture snapshots when lockfiles change'
                            : 'Manual snapshot capture only'}
                        </div>
                      </div>
                    </div>
                    <Button
                      variant={timeMachineSettings.autoWatchEnabled ? 'default' : 'outline'}
                      size="sm"
                      onClick={handleToggleAutoWatch}
                      disabled={isSavingTMSettings}
                      className={cn(
                        timeMachineSettings.autoWatchEnabled && 'bg-cyan-500 hover:bg-cyan-600'
                      )}
                    >
                      {isSavingTMSettings ? (
                        <RefreshCw className="w-4 h-4 animate-spin" />
                      ) : timeMachineSettings.autoWatchEnabled ? (
                        'Enabled'
                      ) : (
                        'Disabled'
                      )}
                    </Button>
                  </div>
                  {/* Debounce setting (only shown when auto-watch is enabled) */}
                  {timeMachineSettings.autoWatchEnabled && (
                    <div className="mt-3 pt-3 border-t border-cyan-500/20 flex items-center justify-between">
                      <div>
                        <span className="text-xs text-muted-foreground">Debounce delay</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min="500"
                          max="10000"
                          step="500"
                          value={timeMachineSettings.debounceMs}
                          onChange={(e) => {
                            const value = Math.max(
                              500,
                              Math.min(10000, parseInt(e.target.value) || 2000)
                            );
                            handleUpdateDebounce(value);
                          }}
                          className="w-20 px-2 py-1 text-sm text-center rounded border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                        <span className="text-xs text-muted-foreground">ms</span>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Storage Stats */}
              {snapshotStats && (
                <div
                  className={cn(
                    'p-4 rounded-lg',
                    'bg-gradient-to-r from-purple-500/5 via-transparent to-transparent',
                    'border border-purple-500/20'
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="p-2.5 rounded-lg bg-purple-500/10 text-purple-500">
                        <Archive className="w-5 h-5" />
                      </div>
                      <div>
                        <div className="text-sm font-medium text-foreground">
                          {snapshotStats.totalSnapshots} Snapshots
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {snapshotStats.totalSizeHuman} total storage
                        </div>
                      </div>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={loadSnapshotStats}
                      className="h-8 w-8"
                      title="Refresh stats"
                    >
                      <RefreshCw className={cn('w-4 h-4', isLoadingStats && 'animate-spin')} />
                    </Button>
                  </div>
                </div>
              )}

              {/* Retention Setting */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between mb-3">
                  <div>
                    <h4 className="text-sm font-medium text-foreground">Snapshot Retention</h4>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Keep the most recent snapshots per project
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">Keep</span>
                    <input
                      type="number"
                      min="1"
                      max="100"
                      value={keepPerProject}
                      onChange={(e) =>
                        setKeepPerProject(Math.max(1, parseInt(e.target.value) || 1))
                      }
                      className="w-16 px-2 py-1 text-sm text-center rounded border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                    <span className="text-xs text-muted-foreground">per project</span>
                  </div>
                </div>
                <Button
                  variant="outline"
                  onClick={handlePruneSnapshots}
                  disabled={isPruning || !snapshotStats || snapshotStats.totalSnapshots === 0}
                  className="w-full"
                >
                  {isPruning ? (
                    <>
                      <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                      Pruning...
                    </>
                  ) : (
                    <>
                      <Trash2 className="w-4 h-4 mr-2" />
                      Prune Old Snapshots
                    </>
                  )}
                </Button>
              </div>

              {/* Cleanup Orphaned Storage */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="mb-3">
                  <h4 className="text-sm font-medium text-foreground">Cleanup Orphaned Storage</h4>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    Remove snapshot files that are no longer referenced in the database
                  </p>
                </div>
                <Button
                  variant="outline"
                  onClick={handleCleanupOrphaned}
                  disabled={isCleaningUp}
                  className="w-full"
                >
                  {isCleaningUp ? (
                    <>
                      <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                      Cleaning up...
                    </>
                  ) : (
                    <>
                      <Trash2 className="w-4 h-4 mr-2" />
                      Cleanup Orphaned Files
                    </>
                  )}
                </Button>
              </div>

              {/* Result Message */}
              {pruneResult && (
                <div
                  className={cn(
                    'p-3 rounded-lg text-sm',
                    pruneResult.count > 0
                      ? 'bg-green-500/10 text-green-600 dark:text-green-400 border border-green-500/20'
                      : 'bg-muted text-muted-foreground border border-border'
                  )}
                >
                  {pruneResult.type === 'prune' ? (
                    pruneResult.count > 0 ? (
                      <>Deleted {pruneResult.count} old snapshots</>
                    ) : (
                      <>No snapshots to prune</>
                    )
                  ) : pruneResult.count > 0 ? (
                    <>Cleaned up {pruneResult.count} orphaned files</>
                  ) : (
                    <>No orphaned files found</>
                  )}
                </div>
              )}
            </div>
          )}
        </SettingSection>

        {/* Data Management Tips */}
        <SettingInfoBox title="Data Management" variant="info">
          <ul className="space-y-1.5">
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>The database location is fixed for WAL mode compatibility</span>
            </li>
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>
                Use <strong>Import/Export</strong> in the Data section to backup or transfer your
                data between devices
              </span>
            </li>
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>
                <code className="px-1 py-0.5 rounded bg-muted text-foreground text-xs">
                  .db-wal
                </code>{' '}
                and{' '}
                <code className="px-1 py-0.5 rounded bg-muted text-foreground text-xs">
                  .db-shm
                </code>{' '}
                files are part of WAL mode - do not delete them separately
              </span>
            </li>
          </ul>
        </SettingInfoBox>
      </div>
    </div>
  );
};

// ============================================================================
// Internal Components
// ============================================================================

/** Loading skeleton for storage location */
const StorageLocationSkeleton: React.FC = () => (
  <div className="space-y-3">
    <div className="p-4 rounded-lg border border-border">
      <div className="flex items-start gap-3">
        <Skeleton className="w-11 h-11 rounded-lg" />
        <div className="flex-1 space-y-2">
          <Skeleton className="w-32 h-4" />
          <Skeleton className="w-64 h-3" />
        </div>
        <div className="flex gap-1">
          <Skeleton className="w-8 h-8 rounded" />
          <Skeleton className="w-8 h-8 rounded" />
        </div>
      </div>
    </div>
    <Skeleton className="w-full h-9 rounded-md" />
  </div>
);

/** Loading skeleton for Time Machine storage section */
const TimeMachineStorageSkeleton: React.FC = () => (
  <div className="space-y-4">
    <div className="p-4 rounded-lg border border-border">
      <div className="flex items-center gap-3">
        <Skeleton className="w-11 h-11 rounded-lg" />
        <div className="space-y-2">
          <Skeleton className="w-28 h-4" />
          <Skeleton className="w-20 h-3" />
        </div>
      </div>
    </div>
    <div className="p-4 rounded-lg border border-border space-y-3">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <Skeleton className="w-32 h-4" />
          <Skeleton className="w-48 h-3" />
        </div>
        <Skeleton className="w-24 h-8" />
      </div>
      <Skeleton className="w-full h-9 rounded-md" />
    </div>
    <div className="p-4 rounded-lg border border-border space-y-3">
      <div className="space-y-1">
        <Skeleton className="w-40 h-4" />
        <Skeleton className="w-64 h-3" />
      </div>
      <Skeleton className="w-full h-9 rounded-md" />
    </div>
  </div>
);

/** Feature card for database features section */
interface FeatureCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  variant?: 'default' | 'success' | 'info';
}

const FeatureCard: React.FC<FeatureCardProps> = ({
  icon,
  title,
  description,
  variant = 'default',
}) => {
  const variantStyles = {
    default: {
      border: 'border-border',
      iconBg: 'bg-muted text-muted-foreground',
    },
    success: {
      border: 'border-green-500/20',
      iconBg: 'bg-green-500/10 text-green-600 dark:text-green-400',
    },
    info: {
      border: 'border-blue-500/20',
      iconBg: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
    },
  };

  const styles = variantStyles[variant];

  return (
    <div className={cn('flex items-start gap-3 p-3 rounded-lg', 'border bg-card', styles.border)}>
      <div className={cn('p-2 rounded-lg flex-shrink-0', styles.iconBg)}>{icon}</div>
      <div className="min-w-0">
        <h4 className="text-sm font-medium text-foreground">{title}</h4>
        <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
      </div>
    </div>
  );
};
