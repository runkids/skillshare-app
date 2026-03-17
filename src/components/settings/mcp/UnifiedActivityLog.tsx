/**
 * UnifiedActivityLog - Merged timeline view of MCP activity
 * Combines Server Logs (tool calls) and Action History into a single timeline
 */

import React, { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Activity,
  Play,
  Globe,
  GitBranch,
  Wrench,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  XCircle,
  Clock,
  Loader2,
  RefreshCw,
  Trash2,
  Filter,
  Search,
  ShieldAlert,
  Terminal,
  Square,
} from 'lucide-react';
import { cn } from '../../../lib/utils';
import { Select } from '../../ui/Select';
import { Button } from '../../ui/Button';
import { useActionHistory } from '../../../hooks/useMCPActions';
import { mcpAPI, type McpLogEntry, type McpLogsResponse } from '../../../lib/tauri-api';
import type { MCPActionExecution, MCPActionType } from '../../../types/mcp-action';

// ============================================================================
// Types
// ============================================================================

/** Unified activity entry that can represent either a tool call or action execution */
interface UnifiedActivityEntry {
  id: string;
  type: 'tool_call' | 'action';
  timestamp: Date;
  name: string;
  status: 'success' | 'denied' | 'error' | 'running' | 'pending';
  durationMs?: number;
  details?: Record<string, unknown>;
  error?: string | null;
  // For tool calls
  toolArguments?: Record<string, unknown>;
  // For actions
  actionType?: MCPActionType;
  result?: unknown;
  sourceClient?: string;
}

interface UnifiedActivityLogProps {
  className?: string;
  maxHeight?: string;
}

// ============================================================================
// Constants
// ============================================================================

const STATUS_CONFIG = {
  success: {
    icon: <CheckCircle2 className="w-3.5 h-3.5" />,
    label: 'Success',
    dotColor: 'bg-emerald-500',
    textColor: 'text-emerald-600 dark:text-emerald-400',
    bgColor: 'bg-emerald-500/10',
  },
  denied: {
    icon: <ShieldAlert className="w-3.5 h-3.5" />,
    label: 'Denied',
    dotColor: 'bg-amber-500',
    textColor: 'text-amber-600 dark:text-amber-400',
    bgColor: 'bg-amber-500/10',
  },
  error: {
    icon: <XCircle className="w-3.5 h-3.5" />,
    label: 'Error',
    dotColor: 'bg-red-500',
    textColor: 'text-red-600 dark:text-red-400',
    bgColor: 'bg-red-500/10',
  },
  running: {
    icon: <Loader2 className="w-3.5 h-3.5 animate-spin" />,
    label: 'Running',
    dotColor: 'bg-blue-500',
    textColor: 'text-blue-600 dark:text-blue-400',
    bgColor: 'bg-blue-500/10',
  },
  pending: {
    icon: <Clock className="w-3.5 h-3.5" />,
    label: 'Pending',
    dotColor: 'bg-slate-400',
    textColor: 'text-slate-600 dark:text-slate-400',
    bgColor: 'bg-slate-500/10',
  },
} as const;

const TYPE_CONFIG = {
  tool_call: {
    icon: <Terminal className="w-3.5 h-3.5" />,
    label: 'Tool Call',
    color: 'text-cyan-500',
    bgColor: 'bg-cyan-500/10',
  },
  action: {
    icon: <Wrench className="w-3.5 h-3.5" />,
    label: 'Action',
    color: 'text-violet-500',
    bgColor: 'bg-violet-500/10',
  },
} as const;

const ACTION_TYPE_ICON: Record<MCPActionType, React.ReactNode> = {
  script: <Play className="w-3 h-3" />,
  webhook: <Globe className="w-3 h-3" />,
  workflow: <GitBranch className="w-3 h-3" />,
};

// ============================================================================
// Helper Functions
// ============================================================================

/** Convert McpLogEntry to UnifiedActivityEntry */
function convertLogEntry(entry: McpLogEntry, index: number): UnifiedActivityEntry {
  let status: UnifiedActivityEntry['status'] = 'success';
  if (entry.result === 'permission_denied') status = 'denied';
  else if (entry.result === 'error') status = 'error';

  return {
    id: `log-${entry.timestamp}-${index}`,
    type: 'tool_call',
    timestamp: new Date(entry.timestamp),
    name: entry.tool,
    status,
    durationMs: entry.durationMs,
    toolArguments: entry.arguments,
    error: entry.error,
  };
}

/** Convert MCPActionExecution to UnifiedActivityEntry */
function convertActionExecution(execution: MCPActionExecution): UnifiedActivityEntry {
  let status: UnifiedActivityEntry['status'] = 'success';
  switch (execution.status) {
    case 'completed':
      status = 'success';
      break;
    case 'failed':
    case 'timed_out':
      status = 'error';
      break;
    case 'denied':
    case 'cancelled':
      status = 'denied';
      break;
    case 'running':
    case 'queued':
      status = 'running';
      break;
    case 'pending_confirm':
      status = 'pending';
      break;
  }

  return {
    id: `action-${execution.id}`,
    type: 'action',
    timestamp: new Date(execution.startedAt),
    name: execution.actionName,
    status,
    durationMs: execution.durationMs,
    actionType: execution.actionType,
    details: execution.parameters,
    result: execution.result,
    error: execution.errorMessage,
    sourceClient: execution.sourceClient,
  };
}

/** Format relative time */
function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return 'Just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString();
}

/** Format duration */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

// ============================================================================
// Activity Row Component
// ============================================================================

interface ActivityRowProps {
  entry: UnifiedActivityEntry;
  onStop?: (entryId: string, toolCallId?: string) => Promise<void>;
}

const ActivityRow: React.FC<ActivityRowProps> = ({ entry, onStop }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const statusConfig = STATUS_CONFIG[entry.status];
  const typeConfig = TYPE_CONFIG[entry.type];

  const hasDetails =
    entry.toolArguments || entry.details || entry.result || entry.error || entry.sourceClient;

  // Extract tool call ID from entry ID (format: "action-{executionId}" or "log-{timestamp}-{index}")
  const toolCallId = entry.type === 'action' ? entry.id.replace('action-', '') : undefined;

  const handleStop = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      if (!onStop) return;

      setIsStopping(true);
      try {
        await onStop(entry.id, toolCallId);
      } finally {
        setIsStopping(false);
      }
    },
    [entry.id, toolCallId, onStop]
  );

  return (
    <div
      className={cn(
        'group border-l-2 pl-4 py-2 relative',
        'hover:bg-muted/30 transition-colors',
        statusConfig.dotColor.replace('bg-', 'border-')
      )}
    >
      {/* Timeline dot */}
      <div
        className={cn('absolute -left-[5px] top-3 w-2 h-2 rounded-full', statusConfig.dotColor)}
      />

      {/* Main content */}
      <div
        className={cn('flex items-start gap-3 cursor-pointer', hasDetails && 'cursor-pointer')}
        onClick={() => hasDetails && setIsExpanded(!isExpanded)}
      >
        {/* Expand indicator */}
        <div className="w-4 h-4 flex items-center justify-center shrink-0 mt-0.5">
          {hasDetails ? (
            isExpanded ? (
              <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-muted-foreground" />
            )
          ) : (
            <div className="w-3.5" />
          )}
        </div>

        {/* Type badge */}
        <div
          className={cn(
            'flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium shrink-0',
            typeConfig.bgColor,
            typeConfig.color
          )}
        >
          {entry.type === 'action' && entry.actionType && ACTION_TYPE_ICON[entry.actionType]
            ? ACTION_TYPE_ICON[entry.actionType]
            : typeConfig.icon}
          <span className="hidden sm:inline">
            {entry.type === 'action' && entry.actionType
              ? entry.actionType.charAt(0).toUpperCase() + entry.actionType.slice(1)
              : typeConfig.label}
          </span>
        </div>

        {/* Name */}
        <code className="flex-1 min-w-0 text-sm font-mono font-medium text-foreground truncate">
          {entry.name}
        </code>

        {/* Status badge */}
        <div
          className={cn(
            'flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium shrink-0',
            statusConfig.bgColor,
            statusConfig.textColor
          )}
        >
          {statusConfig.icon}
          <span className="hidden sm:inline">{statusConfig.label}</span>
        </div>

        {/* Stop button for running processes */}
        {entry.status === 'running' && onStop && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleStop}
            disabled={isStopping}
            className="h-6 px-2 text-xs hover:bg-red-500/10 hover:text-red-500"
          >
            {isStopping ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <>
                <Square className="w-3 h-3 mr-1" />
                Stop
              </>
            )}
          </Button>
        )}

        {/* Duration */}
        {entry.durationMs !== undefined && (
          <span className="text-xs text-muted-foreground tabular-nums shrink-0">
            {formatDuration(entry.durationMs)}
          </span>
        )}

        {/* Time */}
        <span className="text-xs text-muted-foreground shrink-0">
          {formatRelativeTime(entry.timestamp)}
        </span>
      </div>

      {/* Expanded details */}
      {isExpanded && hasDetails && (
        <div className="mt-2 ml-4 space-y-2 text-xs">
          {/* Arguments / Parameters */}
          {(entry.toolArguments !== undefined || entry.details !== undefined) && (
            <div>
              <span className="text-muted-foreground font-medium">
                {entry.type === 'tool_call' ? 'Arguments:' : 'Parameters:'}
              </span>
              <pre className="mt-1 p-2 rounded bg-muted/50 overflow-x-auto max-h-32 text-[11px]">
                {JSON.stringify(entry.toolArguments ?? entry.details, null, 2)}
              </pre>
            </div>
          )}

          {/* Result */}
          {entry.result !== undefined && (
            <div>
              <span className="text-muted-foreground font-medium">Result:</span>
              <pre className="mt-1 p-2 rounded bg-muted/50 overflow-x-auto max-h-32 text-[11px]">
                {JSON.stringify(entry.result, null, 2)}
              </pre>
            </div>
          )}

          {/* Error */}
          {entry.error && (
            <div>
              <span className="text-red-500 font-medium">Error:</span>
              <pre className="mt-1 p-2 rounded bg-red-500/10 text-red-600 dark:text-red-400 overflow-x-auto text-[11px]">
                {entry.error}
              </pre>
            </div>
          )}

          {/* Source client */}
          {entry.sourceClient && (
            <div className="text-muted-foreground">
              <span className="font-medium">Source:</span> <span>{entry.sourceClient}</span>
            </div>
          )}

          {/* Full timestamp */}
          <div className="text-muted-foreground">
            <span className="font-medium">Time:</span>{' '}
            <span>{entry.timestamp.toLocaleString()}</span>
          </div>
        </div>
      )}
    </div>
  );
};

// ============================================================================
// Filter Options
// ============================================================================

const TYPE_FILTER_OPTIONS = [
  { value: 'all', label: 'All Types' },
  { value: 'tool_call', label: 'Tool Calls' },
  { value: 'action', label: 'Actions' },
];

const STATUS_FILTER_OPTIONS = [
  { value: 'all', label: 'All Status' },
  { value: 'success', label: 'Success' },
  { value: 'denied', label: 'Denied' },
  { value: 'error', label: 'Error' },
  { value: 'running', label: 'Running' },
];

// ============================================================================
// Main Component
// ============================================================================

export const UnifiedActivityLog: React.FC<UnifiedActivityLogProps> = ({
  className,
  maxHeight = '450px',
}) => {
  // State
  const [typeFilter, setTypeFilter] = useState<'all' | 'tool_call' | 'action'>('all');
  const [statusFilter, setStatusFilter] = useState<'all' | UnifiedActivityEntry['status']>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [logsResponse, setLogsResponse] = useState<McpLogsResponse | null>(null);
  const [isLoadingLogs, setIsLoadingLogs] = useState(false);

  // Get action history
  const {
    executions,
    isLoading: isLoadingActions,
    refresh: refreshActions,
    cleanup,
  } = useActionHistory({ limit: 100 });

  // Load server logs
  const loadLogs = useCallback(async () => {
    setIsLoadingLogs(true);
    try {
      const response = await mcpAPI.getLogs(100);
      setLogsResponse(response);
    } catch (err) {
      console.error('Failed to load logs:', err);
    } finally {
      setIsLoadingLogs(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  // Listen for background process status changes (for real-time updates)
  const unlistenRef = useRef<UnlistenFn | null>(null);
  useEffect(() => {
    let isMounted = true;

    const setupListener = async () => {
      unlistenRef.current = await listen<{ processId: string; status: string }>(
        'ai:background-process-status',
        () => {
          // Refresh when any process status changes
          if (isMounted) {
            loadLogs();
            refreshActions();
          }
        }
      );
    };

    setupListener();

    return () => {
      isMounted = false;
      unlistenRef.current?.();
    };
  }, [loadLogs, refreshActions]);

  // Merge and sort entries
  const unifiedEntries = useMemo<UnifiedActivityEntry[]>(() => {
    const entries: UnifiedActivityEntry[] = [];

    // Add tool call logs
    if (logsResponse) {
      logsResponse.entries.forEach((entry, index) => {
        entries.push(convertLogEntry(entry, index));
      });
    }

    // Add action executions
    executions.forEach((execution) => {
      entries.push(convertActionExecution(execution));
    });

    // Sort by timestamp (newest first)
    entries.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());

    return entries;
  }, [logsResponse, executions]);

  // Filter entries
  const filteredEntries = useMemo(() => {
    return unifiedEntries.filter((entry) => {
      // Type filter
      if (typeFilter !== 'all' && entry.type !== typeFilter) return false;

      // Status filter
      if (statusFilter !== 'all' && entry.status !== statusFilter) return false;

      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        if (!entry.name.toLowerCase().includes(query)) return false;
      }

      return true;
    });
  }, [unifiedEntries, typeFilter, statusFilter, searchQuery]);

  // Stats
  const stats = useMemo(() => {
    const total = unifiedEntries.length;
    const success = unifiedEntries.filter((e) => e.status === 'success').length;
    const denied = unifiedEntries.filter((e) => e.status === 'denied').length;
    const errors = unifiedEntries.filter((e) => e.status === 'error').length;
    return { total, success, denied, errors };
  }, [unifiedEntries]);

  // Handlers
  const handleRefresh = useCallback(async () => {
    await Promise.all([loadLogs(), refreshActions()]);
  }, [loadLogs, refreshActions]);

  // Stop a running process/execution
  const handleStopExecution = useCallback(
    async (entryId: string, toolCallId?: string) => {
      try {
        // Try to stop via AI Assistant tool execution cancellation
        if (toolCallId) {
          await invoke('ai_assistant_stop_tool_execution', { toolCallId });
        }

        // Also try to stop any background process with this ID
        const processId = toolCallId || entryId;
        try {
          await invoke('ai_assistant_stop_background_process', { processId });
        } catch {
          // Ignore if no background process found
        }

        // Refresh to update status
        await handleRefresh();
      } catch (err) {
        console.error('Failed to stop execution:', err);
      }
    },
    [handleRefresh]
  );

  const handleCleanup = useCallback(async () => {
    if (window.confirm('Clear all activity logs?')) {
      try {
        await Promise.all([
          mcpAPI.clearLogs(),
          cleanup(100, 7), // Keep last 100 or 7 days
        ]);
        setLogsResponse(null);
        await handleRefresh();
      } catch (err) {
        console.error('Failed to cleanup:', err);
      }
    }
  }, [cleanup, handleRefresh]);

  const isLoading = isLoadingLogs || isLoadingActions;

  return (
    <div className={cn('space-y-4', className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">Activity Timeline</span>
          <span className="text-xs text-muted-foreground px-1.5 py-0.5 rounded bg-muted">
            {stats.total} entries
          </span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={handleCleanup}
            className="p-1.5 rounded-md hover:bg-muted/50 transition-colors"
            title="Clear logs"
          >
            <Trash2 className="w-4 h-4 text-muted-foreground" />
          </button>
          <button
            onClick={handleRefresh}
            disabled={isLoading}
            className="p-1.5 rounded-md hover:bg-muted/50 transition-colors"
            title="Refresh"
          >
            <RefreshCw
              className={cn('w-4 h-4 text-muted-foreground', isLoading && 'animate-spin')}
            />
          </button>
        </div>
      </div>

      {/* Stats bar */}
      <div className="flex items-center gap-4 text-xs">
        <span className="flex items-center gap-1.5">
          <span className={cn('w-2 h-2 rounded-full', STATUS_CONFIG.success.dotColor)} />
          <span className="text-muted-foreground">{stats.success} success</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className={cn('w-2 h-2 rounded-full', STATUS_CONFIG.denied.dotColor)} />
          <span className="text-muted-foreground">{stats.denied} denied</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className={cn('w-2 h-2 rounded-full', STATUS_CONFIG.error.dotColor)} />
          <span className="text-muted-foreground">{stats.errors} errors</span>
        </span>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-2 flex-wrap">
        {/* Search */}
        <div className="relative flex-1 min-w-[150px]">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search..."
            className={cn(
              'w-full pl-8 pr-3 py-1.5 text-xs rounded-md',
              'bg-muted/50 border border-border',
              'text-foreground placeholder:text-muted-foreground',
              'focus:outline-none focus:ring-1 focus:ring-ring'
            )}
          />
        </div>

        <div className="flex items-center gap-1.5">
          <Filter className="w-3.5 h-3.5 text-muted-foreground" />
        </div>

        {/* Type filter */}
        <Select
          value={typeFilter}
          onValueChange={(v) => setTypeFilter(v as typeof typeFilter)}
          options={TYPE_FILTER_OPTIONS}
          size="sm"
          className="w-[110px]"
        />

        {/* Status filter */}
        <Select
          value={statusFilter}
          onValueChange={(v) => setStatusFilter(v as typeof statusFilter)}
          options={STATUS_FILTER_OPTIONS}
          size="sm"
          className="w-[110px]"
        />
      </div>

      {/* Timeline */}
      <div
        className="overflow-y-auto border border-border rounded-lg bg-card/30"
        style={{ maxHeight }}
      >
        {isLoading && filteredEntries.length === 0 ? (
          <div className="py-12 text-center">
            <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Loading activity...</p>
          </div>
        ) : filteredEntries.length === 0 ? (
          <div className="py-12 text-center">
            <Activity className="w-8 h-8 text-muted-foreground/50 mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">
              {searchQuery || typeFilter !== 'all' || statusFilter !== 'all'
                ? 'No matching activity found'
                : 'No activity recorded yet'}
            </p>
            {!searchQuery && typeFilter === 'all' && statusFilter === 'all' && (
              <p className="text-xs text-muted-foreground/70 mt-1">
                Activity will appear here when MCP tools are used
              </p>
            )}
          </div>
        ) : (
          <div className="p-3 space-y-0">
            {filteredEntries.map((entry) => (
              <ActivityRow key={entry.id} entry={entry} onStop={handleStopExecution} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default UnifiedActivityLog;
