/**
 * Workflow Toolbar - Enhanced n8n-style toolbar
 * @see specs/001-expo-workflow-automation/spec.md - US1
 */

import {
  ArrowLeft,
  Play,
  Square,
  SkipForward,
  Plus,
  Loader2,
  CheckCircle2,
  Download,
  Upload,
  Webhook,
  ArrowUpFromLine,
  ArrowDownToLine,
  Workflow,
  Terminal,
  History,
  MoreHorizontal,
} from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Dropdown, DropdownItem, DropdownSeparator } from '../ui/Dropdown';
import type { ExecutionStatus } from '../../types/workflow';

interface WorkflowToolbarProps {
  workflowName: string;
  executionStatus?: ExecutionStatus;
  canExecute: boolean;
  isSaving?: boolean;
  saveSuccess?: boolean;
  onExecute: () => void;
  onCancel: () => void;
  onContinue: () => void;
  onNameChange: (name: string) => void;
  onAddNode: () => void;
  onAddTriggerWorkflow?: () => void;
  onImportNode?: () => void;
  onFromTemplate?: () => void;
  onBack?: () => void;
  // Workflow sharing
  onExportWorkflow?: () => void;
  onImportWorkflow?: () => void;
  // Webhook settings
  onWebhookSettings?: () => void;
  hasOutgoingWebhook?: boolean;
  hasIncomingWebhook?: boolean;
  // Execution history
  onHistory?: () => void;
  historyCount?: number;
}

/**
 * Get status badge configuration
 */
function getStatusBadge(status?: ExecutionStatus) {
  switch (status) {
    case 'running':
      return {
        label: 'Running',
        className: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
        icon: Loader2,
        iconClass: 'animate-spin',
      };
    case 'paused':
      return {
        label: 'Paused',
        className: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
        icon: null,
        iconClass: '',
      };
    case 'completed':
      return {
        label: 'Completed',
        className: 'bg-green-500/20 text-green-400 border-green-500/30',
        icon: null,
        iconClass: '',
      };
    case 'failed':
      return {
        label: 'Failed',
        className: 'bg-red-500/20 text-red-400 border-red-500/30',
        icon: null,
        iconClass: '',
      };
    case 'cancelled':
      return {
        label: 'Cancelled',
        className: 'bg-muted text-muted-foreground border-muted',
        icon: null,
        iconClass: '',
      };
    default:
      return null;
  }
}

/**
 * Workflow Toolbar Component
 * Enhanced n8n-style toolbar with execution controls and workflow info
 */
export function WorkflowToolbar({
  workflowName,
  executionStatus,
  canExecute,
  isSaving,
  saveSuccess,
  onExecute,
  onCancel,
  onContinue,
  onNameChange,
  onAddNode,
  onAddTriggerWorkflow,
  onImportNode,
  onBack,
  onExportWorkflow,
  onImportWorkflow,
  onWebhookSettings,
  hasOutgoingWebhook,
  hasIncomingWebhook,
  onHistory,
  historyCount,
}: WorkflowToolbarProps) {
  const hasAnyWebhook = hasOutgoingWebhook || hasIncomingWebhook;
  const isRunning = executionStatus === 'running';
  const isPaused = executionStatus === 'paused';
  const statusBadge = getStatusBadge(executionStatus);

  return (
    <div className="flex items-center justify-between px-4 py-3 bg-secondary">
      {/* Left Section: Back button and workflow name */}
      <div className="flex items-center gap-4 flex-1 min-w-0">
        {/* Back Button */}
        {onBack && (
          <Button
            variant="ghost"
            size="icon"
            onClick={onBack}
            className="shrink-0 text-muted-foreground hover:text-foreground"
          >
            <ArrowLeft className="w-5 h-5" />
          </Button>
        )}

        {/* Workflow Name Input */}
        <div className="flex-1 max-w-md">
          <Input
            value={workflowName}
            onChange={(e) => onNameChange(e.target.value)}
            placeholder="Workflow Name"
            disabled={isRunning}
            className={cn(
              'bg-muted/50 border-border text-foreground font-medium',
              'focus:bg-background focus:border-blue-500',
              'disabled:opacity-50'
            )}
          />
        </div>
      </div>

      {/* Right Section: Actions */}
      <div className="flex items-center gap-2 shrink-0">
        {/* Auto-save Indicator */}
        {isSaving && <Loader2 className="w-4 h-4 text-muted-foreground animate-spin" />}
        {!isSaving && saveSuccess && <CheckCircle2 className="w-4 h-4 text-green-500" />}

        {/* Add Step Dropdown */}
        <Dropdown
          align="right"
          trigger={
            <Button
              variant="outline"
              size="sm"
              disabled={isRunning}
              className="border-border text-foreground hover:bg-accent hover:text-accent-foreground"
            >
              <Plus className="w-4 h-4 mr-1.5" />
              Add Step
            </Button>
          }
        >
          <DropdownItem onClick={onAddNode} icon={<Terminal className="w-4 h-4" />}>
            Script Step
          </DropdownItem>
          {onAddTriggerWorkflow && (
            <DropdownItem onClick={onAddTriggerWorkflow} icon={<Workflow className="w-4 h-4" />}>
              Trigger Workflow
            </DropdownItem>
          )}
          {onImportNode && (
            <DropdownItem onClick={onImportNode} icon={<Upload className="w-4 h-4" />}>
              Import Step
            </DropdownItem>
          )}
        </Dropdown>

        {/* More Dropdown - combines Webhook, Share, and History */}
        {(onWebhookSettings || onExportWorkflow || onImportWorkflow || onHistory) && (
          <Dropdown
            align="right"
            trigger={
              <Button
                variant="outline"
                size="sm"
                disabled={isRunning}
                className={cn(
                  'border-border text-foreground hover:bg-accent hover:text-accent-foreground',
                  hasAnyWebhook && 'border-purple-600'
                )}
              >
                <MoreHorizontal className="w-4 h-4 mr-1.5" />
                More
                {hasAnyWebhook && (
                  <span
                    className="ml-1.5 w-2 h-2 rounded-full bg-purple-500"
                    title="Webhook enabled"
                  />
                )}
              </Button>
            }
          >
            {/* Webhook Settings */}
            {onWebhookSettings && (
              <DropdownItem onClick={onWebhookSettings} icon={<Webhook className="w-4 h-4" />}>
                <span className="flex items-center gap-2">
                  Webhook Settings
                  {hasAnyWebhook && (
                    <span className="flex items-center gap-0.5">
                      {hasOutgoingWebhook && (
                        <span title="Outgoing Webhook enabled" className="text-green-500">
                          <ArrowUpFromLine className="w-3 h-3" />
                        </span>
                      )}
                      {hasIncomingWebhook && (
                        <span title="Incoming Webhook enabled" className="text-purple-400">
                          <ArrowDownToLine className="w-3 h-3" />
                        </span>
                      )}
                    </span>
                  )}
                </span>
              </DropdownItem>
            )}

            {/* Separator between Webhook and Share */}
            {onWebhookSettings && (onExportWorkflow || onImportWorkflow) && <DropdownSeparator />}

            {/* Share options */}
            {onExportWorkflow && (
              <DropdownItem onClick={onExportWorkflow} icon={<Download className="w-4 h-4" />}>
                Export Workflow
              </DropdownItem>
            )}
            {onImportWorkflow && (
              <DropdownItem onClick={onImportWorkflow} icon={<Upload className="w-4 h-4" />}>
                Import Workflow
              </DropdownItem>
            )}

            {/* Separator before History */}
            {(onExportWorkflow || onImportWorkflow) && onHistory && <DropdownSeparator />}

            {/* History */}
            {onHistory && (
              <DropdownItem onClick={onHistory} icon={<History className="w-4 h-4" />}>
                <span className="flex items-center gap-2">
                  Execution History
                  {historyCount !== undefined && historyCount > 0 && (
                    <span className="px-1.5 py-0.5 text-xs bg-muted rounded-full">
                      {historyCount}
                    </span>
                  )}
                </span>
              </DropdownItem>
            )}
          </Dropdown>
        )}

        {/* Divider */}
        <div className="w-px h-6 bg-border mx-1" />

        {/* Status Badge - positioned near execution controls */}
        {statusBadge && (
          <div
            className={cn(
              'flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-full border shrink-0 mr-2',
              statusBadge.className
            )}
          >
            {statusBadge.icon && (
              <statusBadge.icon className={cn('w-3 h-3', statusBadge.iconClass)} />
            )}
            {statusBadge.label}
          </div>
        )}

        {/* Execution Controls */}
        {!isRunning && !isPaused && (
          <Button size="sm" onClick={onExecute} disabled={!canExecute} variant="success">
            <Play className="w-4 h-4 mr-1.5" />
            Run
          </Button>
        )}

        {isRunning && (
          <Button size="sm" onClick={onCancel} variant="destructive">
            <Square className="w-4 h-4 mr-1.5" />
            Stop
          </Button>
        )}

        {isPaused && (
          <>
            <Button variant="default" size="sm" onClick={onContinue}>
              <SkipForward className="w-4 h-4 mr-1.5" />
              Continue
            </Button>
            <Button size="sm" onClick={onCancel} variant="destructive">
              <Square className="w-4 h-4 mr-1.5" />
              Stop
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
