/**
 * Data Settings Panel
 * Import and export PackageFlow data with improved visual hierarchy
 *
 * Features:
 * - Clear separation between Export and Import sections
 * - Visual indicators for data types included
 * - Keyboard shortcut hints
 * - Best practices information
 */

import React, { useState, useEffect, useMemo } from 'react';
import {
  ArrowLeftRight,
  Download,
  Upload,
  FolderDown,
  Workflow,
  GitBranch,
  Settings,
  Zap,
  Shield,
  Clock,
  CheckCircle2,
  Info,
  Archive,
  Bell,
} from 'lucide-react';
import { Button } from '../../ui/Button';
import { SettingSection } from '../ui/SettingSection';
import { SettingInfoBox } from '../ui/SettingInfoBox';
import { Skeleton } from '../../ui/Skeleton';
import { cn } from '../../../lib/utils';
import { settingsAPI } from '../../../lib/tauri-api';

// Keyboard shortcut display helper
const isMac =
  typeof navigator !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
const modKey = isMac ? 'Cmd' : 'Ctrl';

interface DataSettingsPanelProps {
  onExport?: () => void;
  onImport?: () => void;
}

interface DataCounts {
  projects: number;
  workflows: number;
  templates: number;
  isLoading: boolean;
}

export const DataSettingsPanel: React.FC<DataSettingsPanelProps> = ({ onExport, onImport }) => {
  const [dataCounts, setDataCounts] = useState<DataCounts>({
    projects: 0,
    workflows: 0,
    templates: 0,
    isLoading: true,
  });

  // Load data counts on mount
  useEffect(() => {
    const loadCounts = async () => {
      try {
        // Fetch workflows count using settingsAPI
        const workflowsResult = await settingsAPI.loadWorkflows();

        setDataCounts({
          projects: 0,
          workflows: workflowsResult.length,
          templates: 0, // Templates count would need separate API
          isLoading: false,
        });
      } catch (error) {
        console.error('Failed to load data counts:', error);
        setDataCounts((prev) => ({ ...prev, isLoading: false }));
      }
    };
    loadCounts();
  }, []);

  // Calculate total items
  const totalItems = useMemo(() => {
    return dataCounts.projects + dataCounts.workflows + dataCounts.templates;
  }, [dataCounts]);

  return (
    <div className="flex flex-col h-full">
      {/* Fixed Header */}
      <div className="flex-shrink-0 pb-4 border-b border-border bg-background">
        <h2 className="text-xl font-semibold text-foreground flex items-center">
          <ArrowLeftRight className="w-5 h-5 pr-1" />
          Import / Export
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          Backup and restore your PackageFlow data
        </p>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pt-4 space-y-6">
        {/* Export Section */}
        <SettingSection
          title="Export Data"
          description="Create a backup of your configuration and data"
          icon={<Download className="w-4 h-4" />}
        >
          <div
            className={cn(
              'group relative p-4 rounded-lg',
              'bg-gradient-to-r from-green-500/5 via-transparent to-transparent',
              'border border-green-500/20',
              'transition-colors hover:border-green-500/40'
            )}
          >
            <div className="flex items-start gap-3">
              {/* Export Icon */}
              <div
                className={cn(
                  'flex-shrink-0 p-2.5 rounded-lg',
                  'bg-green-500/10 text-green-500 dark:text-green-400'
                )}
              >
                <Archive className="w-5 h-5" />
              </div>

              {/* Export Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-foreground">PackageFlow Backup</span>
                  {totalItems > 0 && (
                    <span
                      className={cn(
                        'inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium',
                        'bg-green-500/10 text-green-600 dark:text-green-400 border border-green-500/20'
                      )}
                    >
                      <CheckCircle2 className="w-3 h-3" />
                      {totalItems} items
                    </span>
                  )}
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  Download all projects, workflows, templates, and settings as a portable backup
                  file
                </p>
              </div>
            </div>

            {/* Export Button */}
            <div className="mt-4">
              <Button variant="outline-success" onClick={onExport} className="w-full">
                <Download className="w-4 h-4 pr-1" />
                Export Data
                <kbd className="ml-2 px-1.5 py-0.5 text-xs font-mono bg-green-500/10 rounded">
                  {modKey}+E
                </kbd>
              </Button>
            </div>
          </div>
        </SettingSection>

        {/* Import Section */}
        <SettingSection
          title="Import Data"
          description="Restore data from a backup file"
          icon={<Upload className="w-4 h-4" />}
        >
          <div
            className={cn(
              'group relative p-4 rounded-lg',
              'bg-gradient-to-r from-blue-500/5 via-transparent to-transparent',
              'border border-blue-500/20',
              'transition-colors hover:border-blue-500/40'
            )}
          >
            <div className="flex items-start gap-3">
              {/* Import Icon */}
              <div
                className={cn(
                  'flex-shrink-0 p-2.5 rounded-lg',
                  'bg-blue-500/10 text-blue-500 dark:text-blue-400'
                )}
              >
                <Upload className="w-5 h-5" />
              </div>

              {/* Import Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-foreground">Restore from Backup</span>
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  Import data from a .packageflow file. Choose to merge with existing data or
                  replace entirely.
                </p>
              </div>
            </div>

            {/* Import Button */}
            <div className="mt-4">
              <Button variant="outline-info" onClick={onImport} className="w-full">
                <Upload className="w-4 h-4 pr-1" />
                Import Data
                <kbd className="ml-2 px-1.5 py-0.5 text-xs font-mono bg-blue-500/10 rounded">
                  {modKey}+I
                </kbd>
              </Button>
            </div>
          </div>
        </SettingSection>

        {/* Data Contents Section */}
        <SettingSection
          title="What's Included"
          description="Data types that will be exported or imported"
          icon={<Shield className="w-4 h-4" />}
        >
          {dataCounts.isLoading ? (
            <DataContentsSkeleton />
          ) : (
            <div className="grid gap-3">
              <DataTypeCard
                icon={<FolderDown className="w-4 h-4" />}
                title="Projects"
                description="Project configurations, paths, and metadata"
                count={dataCounts.projects}
                variant="blue"
              />
              <DataTypeCard
                icon={<Workflow className="w-4 h-4" />}
                title="Workflows"
                description="Automation workflows and step configurations"
                count={dataCounts.workflows}
                variant="purple"
              />
              <DataTypeCard
                icon={<GitBranch className="w-4 h-4" />}
                title="Worktree Templates"
                description="Git worktree templates for branch management"
                variant="green"
              />
              <DataTypeCard
                icon={<Zap className="w-4 h-4" />}
                title="Step Templates"
                description="Reusable workflow step templates"
                variant="cyan"
              />
              <DataTypeCard
                icon={<Bell className="w-4 h-4" />}
                title="Notifications"
                description="Notification preferences and settings"
                variant="amber"
              />
              <DataTypeCard
                icon={<Settings className="w-4 h-4" />}
                title="Application Settings"
                description="Theme, shortcuts, and other preferences"
                variant="default"
              />
            </div>
          )}
        </SettingSection>

        {/* Security Note */}
        <SettingInfoBox title="Security Notice" variant="warning">
          <ul className="space-y-1.5">
            <li className="flex items-start gap-2">
              <Shield className="w-3.5 h-3.5 mt-0.5 flex-shrink-0 text-yellow-500" />
              <span>
                API keys and sensitive credentials are <strong>NOT</strong> included in exports for
                security
              </span>
            </li>
            <li className="flex items-start gap-2">
              <Clock className="w-3.5 h-3.5 mt-0.5 flex-shrink-0 text-yellow-500" />
              <span>After importing, you will need to re-enter any API keys and tokens</span>
            </li>
          </ul>
        </SettingInfoBox>

        {/* Best Practices */}
        <SettingInfoBox title="Best Practices" variant="info">
          <ul className="space-y-1.5">
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>Create regular backups before making major changes to your configuration</span>
            </li>
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>
                Use <strong>Merge</strong> mode when importing to preserve existing data
              </span>
            </li>
            <li className="flex items-start gap-2">
              <Info className="w-3.5 h-3.5 pr-1 mt-0.5 flex-shrink-0 text-blue-500" />
              <span>
                Store backup files securely - they contain your project paths and configurations
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

/** Loading skeleton for data contents */
const DataContentsSkeleton: React.FC = () => (
  <div className="grid gap-3">
    {[1, 2, 3, 4, 5, 6].map((i) => (
      <div key={i} className="p-3 rounded-lg border border-border bg-card">
        <div className="flex items-start gap-3">
          <Skeleton className="w-8 h-8 rounded-lg" />
          <div className="flex-1 space-y-2">
            <Skeleton className="w-24 h-4" />
            <Skeleton className="w-48 h-3" />
          </div>
        </div>
      </div>
    ))}
  </div>
);

/** Data type card component */
interface DataTypeCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  count?: number;
  variant?: 'default' | 'blue' | 'purple' | 'green' | 'cyan' | 'amber';
}

const DataTypeCard: React.FC<DataTypeCardProps> = ({
  icon,
  title,
  description,
  count,
  variant = 'default',
}) => {
  const variantStyles = {
    default: {
      border: 'border-border',
      iconBg: 'bg-muted text-muted-foreground',
      countColor: 'text-foreground',
    },
    blue: {
      border: 'border-blue-500/20',
      iconBg: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
      countColor: 'text-blue-600 dark:text-blue-400',
    },
    purple: {
      border: 'border-purple-500/20',
      iconBg: 'bg-purple-500/10 text-purple-600 dark:text-purple-400',
      countColor: 'text-purple-600 dark:text-purple-400',
    },
    green: {
      border: 'border-green-500/20',
      iconBg: 'bg-green-500/10 text-green-600 dark:text-green-400',
      countColor: 'text-green-600 dark:text-green-400',
    },
    cyan: {
      border: 'border-cyan-500/20',
      iconBg: 'bg-cyan-500/10 text-cyan-600 dark:text-cyan-400',
      countColor: 'text-cyan-600 dark:text-cyan-400',
    },
    amber: {
      border: 'border-amber-500/20',
      iconBg: 'bg-amber-500/10 text-amber-600 dark:text-amber-400',
      countColor: 'text-amber-600 dark:text-amber-400',
    },
  };

  const styles = variantStyles[variant];

  return (
    <div className={cn('flex items-start gap-3 p-3 rounded-lg', 'border bg-card', styles.border)}>
      <div className={cn('p-2 rounded-lg flex-shrink-0', styles.iconBg)}>{icon}</div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2">
          <h4 className="text-sm font-medium text-foreground">{title}</h4>
          {count !== undefined && count > 0 && (
            <span className={cn('text-sm font-medium', styles.countColor)}>{count}</span>
          )}
        </div>
        <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
      </div>
    </div>
  );
};
