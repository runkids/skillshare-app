/**
 * ActionConfirmationDialog Component
 * Floating dialog for approving/denying pending MCP action requests
 * @see specs/021-mcp-actions/spec.md
 */

import React, { useState, useCallback, useEffect } from 'react';
import {
  Play,
  Globe,
  GitBranch,
  Check,
  X,
  AlertTriangle,
  Clock,
  Loader2,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { cn } from '../../../lib/utils';
import { usePendingActions } from '../../../hooks/useMCPActions';
import type { PendingActionRequest } from '../../../lib/tauri-api';

// ============================================================================
// Types
// ============================================================================

interface ActionConfirmationDialogProps {
  /** Position on screen */
  position?: 'bottom-right' | 'bottom-left' | 'top-right' | 'top-left';
  className?: string;
}

// ============================================================================
// Constants
// ============================================================================

const ACTION_TYPE_CONFIG: Record<
  string,
  { icon: React.ReactNode; colorClass: string; bgClass: string; label: string }
> = {
  script: {
    icon: <Play className="w-4 h-4" />,
    colorClass: 'text-emerald-500',
    bgClass: 'bg-emerald-500/10',
    label: 'Script',
  },
  webhook: {
    icon: <Globe className="w-4 h-4" />,
    colorClass: 'text-blue-500',
    bgClass: 'bg-blue-500/10',
    label: 'Webhook',
  },
  workflow: {
    icon: <GitBranch className="w-4 h-4" />,
    colorClass: 'text-purple-500',
    bgClass: 'bg-purple-500/10',
    label: 'Workflow',
  },
};

const POSITION_CLASSES: Record<string, string> = {
  'bottom-right': 'bottom-4 right-4',
  'bottom-left': 'bottom-4 left-4',
  'top-right': 'top-4 right-4',
  'top-left': 'top-4 left-4',
};

// ============================================================================
// Single Request Card Component
// ============================================================================

interface RequestCardProps {
  request: PendingActionRequest;
  onApprove: (executionId: string) => void;
  onDeny: (executionId: string, reason?: string) => void;
  isProcessing: boolean;
}

const RequestCard: React.FC<RequestCardProps> = ({ request, onApprove, onDeny, isProcessing }) => {
  const [showDetails, setShowDetails] = useState(false);
  const [denyReason, setDenyReason] = useState('');
  const [showDenyInput, setShowDenyInput] = useState(false);

  const typeConfig = ACTION_TYPE_CONFIG[request.actionType] || ACTION_TYPE_CONFIG.script;

  // Format time
  const timeAgo = React.useMemo(() => {
    const date = new Date(request.startedAt);
    const now = new Date();
    const diffSec = Math.floor((now.getTime() - date.getTime()) / 1000);
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    return `${Math.floor(diffMin / 60)}h ago`;
  }, [request.startedAt]);

  const handleDenyClick = () => {
    if (showDenyInput && denyReason.trim()) {
      onDeny(request.executionId, denyReason.trim());
    } else if (showDenyInput) {
      onDeny(request.executionId);
    } else {
      setShowDenyInput(true);
    }
  };

  return (
    <div
      className={cn(
        'rounded-lg border border-amber-500/30 bg-amber-500/5',
        'overflow-hidden transition-all duration-200'
      )}
    >
      {/* Header */}
      <div className="p-3">
        <div className="flex items-start gap-3">
          {/* Type icon */}
          <span className={cn('p-2 rounded-lg', typeConfig.bgClass, typeConfig.colorClass)}>
            {typeConfig.icon}
          </span>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm truncate">{request.actionName}</span>
              <span
                className={cn(
                  'text-[10px] px-1.5 py-0.5 rounded',
                  typeConfig.bgClass,
                  typeConfig.colorClass
                )}
              >
                {typeConfig.label}
              </span>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
              {request.description}
            </p>
            <div className="flex items-center gap-2 mt-1 text-[10px] text-muted-foreground">
              <Clock className="w-3 h-3" />
              <span>{timeAgo}</span>
              {request.sourceClient && (
                <>
                  <span className="opacity-50">|</span>
                  <span>from {request.sourceClient}</span>
                </>
              )}
            </div>
          </div>

          {/* Expand button */}
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="p-1 rounded hover:bg-muted/50 transition-colors"
          >
            {showDetails ? (
              <ChevronUp className="w-4 h-4 text-muted-foreground" />
            ) : (
              <ChevronDown className="w-4 h-4 text-muted-foreground" />
            )}
          </button>
        </div>

        {/* Expanded details */}
        {showDetails && request.parameters && Object.keys(request.parameters).length > 0 && (
          <div className="mt-3 pt-3 border-t border-border/50">
            <span className="text-xs text-muted-foreground">Parameters:</span>
            <pre className="mt-1 p-2 rounded bg-muted/50 text-xs overflow-x-auto max-h-24">
              {JSON.stringify(request.parameters, null, 2)}
            </pre>
          </div>
        )}

        {/* Deny reason input */}
        {showDenyInput && (
          <div className="mt-3 pt-3 border-t border-border/50">
            <input
              type="text"
              placeholder="Reason for denial (optional)"
              value={denyReason}
              onChange={(e) => setDenyReason(e.target.value)}
              className="w-full px-2 py-1 text-xs rounded border border-border bg-background"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleDenyClick();
                } else if (e.key === 'Escape') {
                  setShowDenyInput(false);
                  setDenyReason('');
                }
              }}
            />
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex border-t border-border/50">
        <button
          onClick={() => onApprove(request.executionId)}
          disabled={isProcessing}
          className={cn(
            'flex-1 flex items-center justify-center gap-2 py-2 px-3',
            'text-sm font-medium text-emerald-600 dark:text-emerald-400',
            'hover:bg-emerald-500/10 transition-colors',
            'border-r border-border/50',
            isProcessing && 'opacity-50 cursor-not-allowed'
          )}
        >
          {isProcessing ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Check className="w-4 h-4" />
          )}
          Approve
        </button>
        <button
          onClick={handleDenyClick}
          disabled={isProcessing}
          className={cn(
            'flex-1 flex items-center justify-center gap-2 py-2 px-3',
            'text-sm font-medium text-red-600 dark:text-red-400',
            'hover:bg-red-500/10 transition-colors',
            isProcessing && 'opacity-50 cursor-not-allowed'
          )}
        >
          {isProcessing ? <Loader2 className="w-4 h-4 animate-spin" /> : <X className="w-4 h-4" />}
          {showDenyInput ? 'Confirm Deny' : 'Deny'}
        </button>
      </div>
    </div>
  );
};

// ============================================================================
// Main Component
// ============================================================================

export const ActionConfirmationDialog: React.FC<ActionConfirmationDialogProps> = ({
  position = 'bottom-right',
  className,
}) => {
  const { requests, hasPending, pendingCount, approve, deny } = usePendingActions({
    activePollInterval: 3000, // Fast polling when there ARE pending requests
    idlePollInterval: 30000, // Slow polling when idle (save power)
  });

  const [processingIds, setProcessingIds] = useState<Set<string>>(new Set());
  const [isMinimized, setIsMinimized] = useState(false);

  // Handle approve
  const handleApprove = useCallback(
    async (executionId: string) => {
      setProcessingIds((prev) => new Set(prev).add(executionId));
      try {
        await approve(executionId);
      } finally {
        setProcessingIds((prev) => {
          const next = new Set(prev);
          next.delete(executionId);
          return next;
        });
      }
    },
    [approve]
  );

  // Handle deny
  const handleDeny = useCallback(
    async (executionId: string, reason?: string) => {
      setProcessingIds((prev) => new Set(prev).add(executionId));
      try {
        await deny(executionId, reason);
      } finally {
        setProcessingIds((prev) => {
          const next = new Set(prev);
          next.delete(executionId);
          return next;
        });
      }
    },
    [deny]
  );

  // Auto-expand when new requests arrive
  useEffect(() => {
    if (hasPending && isMinimized) {
      setIsMinimized(false);
    }
  }, [hasPending, isMinimized]);

  // Don't render if no pending requests
  if (!hasPending) {
    return null;
  }

  return (
    <div
      className={cn(
        'fixed z-50',
        POSITION_CLASSES[position],
        'w-[360px] max-h-[80vh]',
        'bg-background border border-border rounded-xl shadow-2xl',
        'overflow-hidden',
        className
      )}
    >
      {/* Header */}
      <div
        className={cn(
          'flex items-center gap-2 px-4 py-3',
          'bg-amber-500/10 border-b border-amber-500/20',
          'cursor-pointer'
        )}
        onClick={() => setIsMinimized(!isMinimized)}
      >
        <AlertTriangle className="w-4 h-4 text-amber-500" />
        <span className="flex-1 font-medium text-sm">Action Confirmation Required</span>
        <span className="px-2 py-0.5 rounded-full bg-amber-500 text-white text-xs font-medium">
          {pendingCount}
        </span>
        {isMinimized ? (
          <ChevronUp className="w-4 h-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="w-4 h-4 text-muted-foreground" />
        )}
      </div>

      {/* Content */}
      {!isMinimized && (
        <div className="p-3 space-y-2 overflow-y-auto max-h-[60vh]">
          {requests.map((request) => (
            <RequestCard
              key={request.executionId}
              request={request}
              onApprove={handleApprove}
              onDeny={handleDeny}
              isProcessing={processingIds.has(request.executionId)}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default ActionConfirmationDialog;
