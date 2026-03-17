/**
 * Workflow Editor - Enhanced Visual Workflow Editor
 * n8n-style workflow editor with React Flow canvas and side panel
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { VisualCanvas } from './VisualCanvas';
import { WorkflowToolbar } from './WorkflowToolbar';
import { NodePanel } from './NodePanel';
import { TriggerWorkflowPanel } from './TriggerWorkflowPanel';
import { TemplateSelector } from './TemplateSelector';
import { TerminalOutput } from '../terminal/TerminalOutput';
import { ExecutionHistoryPanel } from './ExecutionHistoryPanel';
import { useWorkflow } from '../../hooks/useWorkflow';
import { useTerminal, useExecutionListener } from '../../hooks/useTerminal';
import { useExecutionHistoryContext } from '../../contexts/ExecutionHistoryContext';
import { useSettings } from '../../contexts/SettingsContext';
import { Dialog, DialogContent } from '../ui/Dialog';
import { SaveAsTemplateDialog } from './SaveAsTemplateDialog';
import { WebhookSettingsDialog } from './WebhookSettingsDialog';
import type { WebhookConfig } from '../../types/webhook';
import type { IncomingWebhookConfig } from '../../types/incoming-webhook';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { cn } from '../../lib/utils';
import {
  Terminal,
  ChevronDown,
  ChevronUp,
  Plus,
  Play,
  FolderOpen,
  Trash2,
  LayoutTemplate,
  PenLine,
  Star,
} from 'lucide-react';
import { resolveCommand, saveNodeAsTemplate } from '../../data/step-templates';
import { settingsAPI, workflowAPI } from '../../lib/tauri-api';
import { useWorkflowExecutionContext } from '../../contexts/WorkflowExecutionContext';
import type {
  Workflow,
  WorkflowNode,
  TriggerWorkflowConfig,
  NodeConfig,
} from '../../types/workflow';
import { isScriptNodeConfig } from '../../types/workflow';

// TODO: export-import removed during feature cleanup, re-implement when needed
const exportWorkflow = async (_w: unknown) => ({
  success: false as boolean,
  error: 'NOT_IMPLEMENTED',
  filePath: '',
});
const importWorkflow = async () => ({
  success: false as boolean,
  error: 'NOT_IMPLEMENTED',
  workflow: null as Workflow | null,
});
const exportNode = async (_n: unknown) => ({
  success: false as boolean,
  error: 'NOT_IMPLEMENTED',
  filePath: '',
});
const importNode = async () => ({
  success: false as boolean,
  error: 'NOT_IMPLEMENTED',
  node: null as WorkflowNode | null,
});
import type { StepTemplate } from '../../types/step-template';

interface NewNodeDialogProps {
  isOpen: boolean;
  defaultCwd?: string;
  insertIndex?: number;
  packageManager?: string;
  onClose: () => void;
  onSave: (
    name: string,
    command: string,
    cwd?: string,
    insertIndex?: number,
    saveAsTemplate?: boolean
  ) => void;
}

type DialogTab = 'templates' | 'custom';

function containsRmCommand(cmd: string): boolean {
  const trimmed = cmd.trim();
  return trimmed.startsWith('rm ') || trimmed === 'rm' || /\|\s*rm(\s|$)/.test(trimmed);
}

function NewNodeDialog({
  isOpen,
  defaultCwd,
  insertIndex,
  packageManager = 'npm',
  onClose,
  onSave,
}: NewNodeDialogProps) {
  const [activeTab, setActiveTab] = useState<DialogTab>('templates');
  const [name, setName] = useState('');
  const [command, setCommand] = useState('');
  const [cwd, setCwd] = useState(defaultCwd ?? '');
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null);
  const [saveAsTemplate, setSaveAsTemplate] = useState(false);

  const hasRmCommand = containsRmCommand(command);

  useEffect(() => {
    if (!isOpen) {
      setName('');
      setCommand('');
      setCwd(defaultCwd ?? '');
      setSelectedTemplateId(null);
      setActiveTab('templates');
      setSaveAsTemplate(false);
    }
  }, [isOpen, defaultCwd]);

  const handleSelectTemplate = useCallback(
    (template: StepTemplate) => {
      setSelectedTemplateId(template.id);
      setName(template.name);
      setCommand(resolveCommand(template.command, packageManager));
      setActiveTab('custom');
    },
    [packageManager]
  );

  const handleSave = () => {
    if (!name.trim() || !command.trim()) return;
    onSave(name.trim(), command.trim(), cwd.trim() || undefined, insertIndex, saveAsTemplate);
    onClose();
  };

  const isInsertMode = insertIndex !== undefined;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSave();
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent
        className={cn(
          'bg-background border-blue-500/30 max-w-2xl max-h-[90vh] p-0 overflow-hidden',
          'shadow-2xl shadow-black/60',
          'flex flex-col'
        )}
      >
        {/* Header with gradient background and icon badge */}
        <div
          className={cn(
            'relative px-6 py-5',
            'border-b border-border',
            'bg-gradient-to-r',
            'dark:from-blue-500/15 dark:via-blue-600/5 dark:to-transparent',
            'from-blue-500/10 via-blue-600/5 to-transparent'
          )}
        >
          <div className="flex items-center gap-4">
            {/* Icon badge */}
            <div
              className={cn(
                'flex-shrink-0',
                'w-12 h-12 rounded-xl',
                'flex items-center justify-center',
                'bg-background/80 dark:bg-background/50 backdrop-blur-sm',
                'border',
                'bg-blue-500/10 border-blue-500/20',
                'shadow-lg'
              )}
            >
              <Plus className="w-6 h-6 text-blue-400" />
            </div>
            <div className="flex-1 min-w-0">
              <h2 className="text-lg font-semibold text-foreground leading-tight">
                {isInsertMode ? `Insert Step at Position ${insertIndex + 1}` : 'Add New Step'}
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                {isInsertMode
                  ? 'Insert a new step into your workflow at the specified position'
                  : 'Choose a template or create a custom step for your workflow'}
              </p>
            </div>
          </div>
        </div>

        {/* Tab Buttons - Enhanced design */}
        <div className="flex gap-2 px-6 py-3 border-b border-border bg-card/30">
          <Button
            type="button"
            variant="ghost"
            onClick={() => setActiveTab('templates')}
            className={cn(
              'flex-1 px-4 py-2.5 text-sm font-medium flex items-center justify-center gap-2',
              'rounded-lg transition-all duration-150',
              activeTab === 'templates'
                ? 'bg-blue-600/20 text-blue-400 ring-1 ring-blue-500/50'
                : 'bg-secondary/50 text-muted-foreground hover:text-foreground hover:bg-secondary'
            )}
          >
            <LayoutTemplate className="w-4 h-4" />
            Templates
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => setActiveTab('custom')}
            className={cn(
              'flex-1 px-4 py-2.5 text-sm font-medium flex items-center justify-center gap-2',
              'rounded-lg transition-all duration-150',
              activeTab === 'custom'
                ? 'bg-blue-600/20 text-blue-400 ring-1 ring-blue-500/50'
                : 'bg-secondary/50 text-muted-foreground hover:text-foreground hover:bg-secondary'
            )}
          >
            <PenLine className="w-4 h-4" />
            Custom
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto min-h-0 px-6 py-4">
          {activeTab === 'templates' ? (
            <div>
              <TemplateSelector
                selectedTemplateId={selectedTemplateId}
                onSelectTemplate={handleSelectTemplate}
              />
            </div>
          ) : (
            <div className="space-y-4" onKeyDown={handleKeyDown}>
              {selectedTemplateId && (
                <div className="flex items-center gap-2 px-3 py-2 bg-blue-600/10 border border-blue-600/30 rounded-lg">
                  <LayoutTemplate className="w-4 h-4 text-blue-400" />
                  <span className="text-xs text-blue-400 dark:text-blue-300">
                    Based on template - feel free to customize
                  </span>
                </div>
              )}

              <div className="space-y-2">
                <label className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <Terminal className="w-4 h-4" />
                  Step Name
                </label>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., Build Project"
                  autoFocus
                  className="bg-background border-border text-foreground"
                />
              </div>

              <div className="space-y-2">
                <label className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <Play className="w-4 h-4" />
                  Shell Command
                </label>
                <textarea
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  placeholder="e.g., npm run build"
                  rows={3}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  data-form-type="other"
                  className={cn(
                    'w-full px-3 py-2 rounded-md border bg-background border-border',
                    'text-foreground placeholder-muted-foreground font-mono text-sm',
                    'focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent',
                    'resize-none'
                  )}
                />
                {hasRmCommand && (
                  <div className="flex items-start gap-2 mt-2 p-2.5 rounded-lg bg-amber-500/10 dark:bg-amber-900/30 border border-amber-500/30 dark:border-amber-700/50">
                    <Trash2 className="w-4 h-4 text-amber-600 dark:text-amber-400 mt-0.5 shrink-0" />
                    <p className="text-xs text-amber-700 dark:text-amber-300">
                      <span className="font-medium">Safe Delete:</span> Files will be moved to Trash
                      instead of being permanently deleted.
                    </p>
                  </div>
                )}
              </div>

              <div className="space-y-2">
                <label className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <FolderOpen className="w-4 h-4" />
                  Working Directory
                  <span className="text-muted-foreground font-normal">(Optional)</span>
                </label>
                <Input
                  value={cwd}
                  onChange={(e) => setCwd(e.target.value)}
                  placeholder="e.g., ~/Developer/project"
                  className="bg-background border-border text-foreground font-mono text-sm"
                />
              </div>

              <Button
                type="button"
                variant="ghost"
                onClick={() => setSaveAsTemplate(!saveAsTemplate)}
                className={cn(
                  'flex items-center gap-3 px-3 py-2.5 rounded-lg border cursor-pointer transition-all w-full text-left h-auto',
                  saveAsTemplate
                    ? 'border-yellow-500/50 bg-yellow-500/10'
                    : 'border-border bg-muted/50 hover:border-accent hover:bg-accent/50'
                )}
              >
                <div
                  className={cn(
                    'w-5 h-5 rounded flex items-center justify-center transition-colors',
                    saveAsTemplate ? 'bg-yellow-500' : 'bg-muted'
                  )}
                >
                  <Star
                    className={cn(
                      'w-3 h-3',
                      saveAsTemplate ? 'text-background' : 'text-muted-foreground'
                    )}
                  />
                </div>
                <div className="flex-1">
                  <span
                    className={cn(
                      'text-sm',
                      saveAsTemplate ? 'text-yellow-600 dark:text-yellow-400' : 'text-foreground'
                    )}
                  >
                    Save as Template
                  </span>
                </div>
                <span className="text-xs text-muted-foreground">Reuse later</span>
              </Button>
            </div>
          )}
        </div>

        {/* Footer with actions */}
        <div
          className={cn(
            'px-6 py-4',
            'border-t border-border',
            'bg-card/50',
            'flex justify-end gap-3',
            'flex-shrink-0'
          )}
        >
          <Button
            variant="ghost"
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground"
          >
            Cancel
          </Button>
          <Button variant="default" onClick={handleSave} disabled={!name.trim() || !command.trim()}>
            {isInsertMode ? 'Insert Step' : 'Add Step'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

interface WorkflowEditorProps {
  initialWorkflow?: Workflow;
  defaultCwd?: string;
  onBack?: () => void;
  onSaved?: (workflow: Workflow) => void;
}

/**
 * Workflow Editor Component
 * Enhanced visual workflow editor with n8n-like experience
 */
export function WorkflowEditor({
  initialWorkflow,
  defaultCwd,
  onBack,
  onSaved,
}: WorkflowEditorProps) {
  const {
    workflow,
    isSaving,
    saveSuccess,
    executionStatus,
    nodeStatuses,
    childProgressMap, // Feature 013
    loadWorkflow,
    setName,
    updateWorkflow,
    addNode,
    addTriggerWorkflowNode,
    updateNode,
    updateNodePosition,
    deleteNode,
    insertNodeAt,
    duplicateNode,
    reorderByPosition,
    save, // For Cmd+S shortcut
    cancel,
    continueExecution,
    onSaved: onSavedRef,
  } = useWorkflow();

  const terminal = useTerminal();
  // Use initialWorkflow.id for filtering - it's available immediately from props
  // workflow.id might be null initially until loadWorkflow is called
  useExecutionListener(terminal, initialWorkflow?.id ?? workflow?.id ?? null);

  // Also use WorkflowExecutionContext for global state synchronization
  const {
    executeWorkflow: executeViaContext,
    cancelExecution: cancelViaContext,
    getExecutionState,
  } = useWorkflowExecutionContext();

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [isNodePanelOpen, setIsNodePanelOpen] = useState(false);
  const [isNewNodeDialogOpen, setIsNewNodeDialogOpen] = useState(false);
  const [insertPosition, setInsertPosition] = useState<number | undefined>(undefined);
  const [isTerminalExpanded, setIsTerminalExpanded] = useState(false);
  const { terminalHeight, setTerminalHeight: saveTerminalHeight } = useSettings();
  const [localTerminalHeight, setLocalTerminalHeight] = useState(terminalHeight);
  const [isResizing, setIsResizing] = useState(false);
  const [isSaveTemplateDialogOpen, setIsSaveTemplateDialogOpen] = useState(false);
  const [nodeToSaveAsTemplate, setNodeToSaveAsTemplate] = useState<string | null>(null);
  const [isWebhookDialogOpen, setIsWebhookDialogOpen] = useState(false);
  const [isHistoryPanelOpen, setIsHistoryPanelOpen] = useState(false);

  // Execution history
  const { getHistory, refreshHistory } = useExecutionHistoryContext();
  const historyCount = workflow ? getHistory(workflow.id).length : 0;

  // Track execution completion to refresh history count
  const lastFinishedAtRef = useRef<string | null>(null);
  const executionState = workflow ? getExecutionState(workflow.id) : null;
  const finishedAt = executionState?.finishedAt?.toISOString() || null;

  useEffect(() => {
    if (!workflow) return;

    // Skip if finishedAt hasn't changed or is null
    if (!finishedAt || finishedAt === lastFinishedAtRef.current) {
      lastFinishedAtRef.current = finishedAt;
      return;
    }

    // finishedAt changed - new execution completed
    lastFinishedAtRef.current = finishedAt;

    // Delay to ensure backend has saved the history
    const timer = setTimeout(() => {
      refreshHistory(workflow.id);
    }, 300);

    return () => clearTimeout(timer);
  }, [finishedAt, workflow, refreshHistory]);

  const selectedNode = selectedNodeId
    ? (workflow?.nodes.find((n) => n.id === selectedNodeId) ?? null)
    : null;

  useEffect(() => {
    if (initialWorkflow) {
      loadWorkflow(initialWorkflow);

      // Restore output history from backend buffer when switching workflows
      const restoreOutput = async () => {
        try {
          const outputResponse = await workflowAPI.getWorkflowOutput(initialWorkflow.id);
          if (outputResponse.found && outputResponse.lines.length > 0) {
            // Convert backend output lines to terminal format
            const restoredLines = outputResponse.lines.map((line) => ({
              type: line.stream as 'stdout' | 'stderr' | 'system',
              content: line.content,
              timestamp: line.timestamp,
              nodeId: line.nodeId,
            }));
            terminal.setLines(restoredLines);
            // Expand terminal if there's output to show
            if (restoredLines.length > 0) {
              setIsTerminalExpanded(true);
            }
          } else {
            // No output history, clear terminal
            terminal.clear();
          }
        } catch (error) {
          console.error('[WorkflowEditor] Failed to restore output:', error);
          terminal.clear();
        }
      };

      restoreOutput();
    }
  }, [initialWorkflow?.id]);

  useEffect(() => {
    onSavedRef.current = onSaved ?? null;
    return () => {
      onSavedRef.current = null;
    };
  }, [onSaved, onSavedRef]);

  useEffect(() => {
    const handleShortcutSaveWorkflow = async () => {
      if (workflow) {
        const savedWorkflow = await save();
        if (savedWorkflow && onSaved) {
          onSaved(savedWorkflow);
        }
      }
    };

    window.addEventListener('shortcut-save-workflow', handleShortcutSaveWorkflow);
    return () => window.removeEventListener('shortcut-save-workflow', handleShortcutSaveWorkflow);
  }, [workflow, save, onSaved]);

  const handleSelectNode = useCallback((nodeId: string | null) => {
    setSelectedNodeId(nodeId);
    if (nodeId) {
      setIsNodePanelOpen(true);
    }
  }, []);

  const handleEditNode = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
    setIsNodePanelOpen(true);
  }, []);

  const handleAddNode = useCallback(() => {
    setInsertPosition(undefined);
    setIsNewNodeDialogOpen(true);
  }, []);

  const handleAddTriggerWorkflow = useCallback(() => {
    addTriggerWorkflowNode('Trigger Workflow', '', '');

    setTimeout(() => {
      if (workflow) {
        const newNode = workflow.nodes.find(
          (n) => n.type === 'trigger-workflow' && n.name === 'Trigger Workflow'
        );
        if (newNode) {
          setSelectedNodeId(newNode.id);
          setIsNodePanelOpen(true);
        }
      }
    }, 100);
  }, [addTriggerWorkflowNode, workflow]);

  const handleInsertNode = useCallback((insertIndex: number) => {
    setInsertPosition(insertIndex);
    setIsNewNodeDialogOpen(true);
  }, []);

  const handleSaveNewNode = useCallback(
    async (
      name: string,
      command: string,
      cwd?: string,
      insertIndex?: number,
      shouldSaveAsTemplate?: boolean
    ) => {
      if (insertIndex !== undefined) {
        insertNodeAt(name, command, insertIndex, cwd);
      } else {
        addNode(name, command, cwd);
      }

      if (shouldSaveAsTemplate) {
        const tempNode = {
          id: 'temp',
          name,
          type: 'script' as const,
          order: 0,
          config: { command, cwd },
        };
        const result = await saveNodeAsTemplate(tempNode, name, 'custom');
        if (result) {
          terminal.addSystemMessage(`Template saved: "${name}"`);
        }
      }
    },
    [addNode, insertNodeAt, terminal]
  );

  const handleCloseNewNodeDialog = useCallback(() => {
    setIsNewNodeDialogOpen(false);
    setInsertPosition(undefined);
  }, []);

  const handleInsertNodeBefore = useCallback(
    (nodeId: string) => {
      const node = workflow?.nodes.find((n) => n.id === nodeId);
      if (node) {
        setInsertPosition(node.order);
        setIsNewNodeDialogOpen(true);
      }
    },
    [workflow]
  );

  const handleInsertNodeAfter = useCallback(
    (nodeId: string) => {
      const node = workflow?.nodes.find((n) => n.id === nodeId);
      if (node) {
        setInsertPosition(node.order + 1);
        setIsNewNodeDialogOpen(true);
      }
    },
    [workflow]
  );

  const handleDuplicateNode = useCallback(
    (nodeId: string) => {
      duplicateNode(nodeId);
    },
    [duplicateNode]
  );

  const handleUpdateNodeFromPanel = useCallback(
    (nodeId: string, updates: { name: string; config: NodeConfig }) => {
      updateNode(nodeId, updates);
    },
    [updateNode]
  );

  const handleUpdateTriggerNodeFromPanel = useCallback(
    (nodeId: string, updates: { name: string; config: TriggerWorkflowConfig }) => {
      updateNode(nodeId, updates);
    },
    [updateNode]
  );

  const handleDeleteNode = useCallback(
    (nodeId: string) => {
      deleteNode(nodeId);
      if (selectedNodeId === nodeId) {
        setSelectedNodeId(null);
        setIsNodePanelOpen(false);
      }
    },
    [deleteNode, selectedNodeId]
  );

  const handleNodePositionChange = useCallback(
    async (nodeId: string, position: { x: number; y: number }) => {
      const updatedWorkflow = await updateNodePosition(nodeId, position);
      if (updatedWorkflow && onSavedRef.current) {
        onSavedRef.current(updatedWorkflow);
      }
    },
    [updateNodePosition, onSavedRef]
  );

  const handleClosePanel = useCallback(() => {
    setIsNodePanelOpen(false);
  }, []);

  const handleExecute = useCallback(async () => {
    if (!workflow || workflow.nodes.length === 0) {
      terminal.addSystemMessage('No steps in workflow');
      return;
    }

    terminal.clear();
    terminal.addSystemMessage(`Starting workflow: ${workflow.name}`);
    setIsTerminalExpanded(true);

    // Execute via Context for global state synchronization
    // This ensures Project page can see the execution status and output
    // useWorkflow will also receive events via its own listeners for local UI state
    await executeViaContext(workflow.id, workflow.nodes.length);
  }, [workflow, executeViaContext, terminal]);

  const handleCancel = useCallback(async () => {
    terminal.addSystemMessage('Stopping execution...');
    // Cancel via both useWorkflow and Context
    await Promise.all([cancel(), workflow?.id ? cancelViaContext(workflow.id) : Promise.resolve()]);
    terminal.addSystemMessage('Execution stopped');
  }, [cancel, cancelViaContext, workflow?.id, terminal]);

  const handleContinue = useCallback(async () => {
    terminal.addSystemMessage('Continuing execution (skipping failed step)...');
    await continueExecution();
  }, [continueExecution, terminal]);

  const handleToggleTerminal = useCallback(() => {
    setIsTerminalExpanded((prev) => !prev);
  }, []);

  // Sync local height with settings when settings load
  useEffect(() => {
    setLocalTerminalHeight(terminalHeight);
  }, [terminalHeight]);

  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsResizing(true);

      const startY = e.clientY;
      const startHeight = localTerminalHeight;

      const handleMouseMove = (moveEvent: MouseEvent) => {
        const deltaY = startY - moveEvent.clientY;
        const newHeight = Math.min(Math.max(startHeight + deltaY, 100), 600);
        setLocalTerminalHeight(newHeight);
      };

      const handleMouseUp = () => {
        setIsResizing(false);
        // Save to persistent settings
        saveTerminalHeight(localTerminalHeight);
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    },
    [localTerminalHeight, saveTerminalHeight]
  );

  const handleExportWorkflow = useCallback(async () => {
    if (!workflow) return;
    const result = await exportWorkflow(workflow);
    if (result.success) {
      terminal.addSystemMessage(`Workflow exported to: ${result.filePath}`);
    } else if (result.error !== 'USER_CANCELLED') {
      terminal.addSystemMessage(`Export failed: ${result.error}`);
    }
  }, [workflow, terminal]);

  const handleImportWorkflow = useCallback(async () => {
    const result = await importWorkflow();
    if (result.success && result.workflow) {
      // Save the imported workflow
      const imported = result.workflow;
      const workflows = await settingsAPI.loadWorkflows();
      await settingsAPI.saveWorkflows([...workflows, imported]);
      terminal.addSystemMessage(
        `Imported workflow: "${imported.name}" (${imported.nodes.length} steps)`
      );
      loadWorkflow(imported);
      if (onSaved) {
        onSaved(imported);
      }
    } else if (result.error && result.error !== 'USER_CANCELLED') {
      terminal.addSystemMessage(`Import failed: ${result.error}`);
    }
  }, [terminal, loadWorkflow, onSaved]);

  const handleExportNode = useCallback(
    async (nodeId: string) => {
      const node = workflow?.nodes.find((n) => n.id === nodeId);
      if (!node) return;

      const result = await exportNode(node);
      if (result.success) {
        terminal.addSystemMessage(`Step exported to: ${result.filePath}`);
      } else if (result.error !== 'USER_CANCELLED') {
        terminal.addSystemMessage(`Export failed: ${result.error}`);
      }
    },
    [workflow, terminal]
  );

  const handleImportNodeFromToolbar = useCallback(async () => {
    const result = await importNode();
    if (result.success && result.node) {
      if (isScriptNodeConfig(result.node.config)) {
        addNode(result.node.name, result.node.config.command, result.node.config.cwd);
        terminal.addSystemMessage(`Imported step: "${result.node.name}"`);
      } else {
        terminal.addSystemMessage('Import failed: Only script nodes can be imported');
      }
    } else if (result.error && result.error !== 'USER_CANCELLED') {
      terminal.addSystemMessage(`Import failed: ${result.error}`);
    }
  }, [terminal, addNode]);

  const handleSaveAsTemplate = useCallback(
    (nodeId: string) => {
      const node = workflow?.nodes.find((n) => n.id === nodeId);
      if (!node) return;
      setNodeToSaveAsTemplate(nodeId);
      setIsSaveTemplateDialogOpen(true);
    },
    [workflow]
  );

  const handleSaveTemplateConfirm = useCallback(
    async (name: string) => {
      const node = nodeToSaveAsTemplate
        ? workflow?.nodes.find((n) => n.id === nodeToSaveAsTemplate)
        : null;
      if (!node) return;

      const result = await saveNodeAsTemplate(node, name, 'custom');
      if (result) {
        terminal.addSystemMessage(`Template saved: "${name}"`);
      } else {
        terminal.addSystemMessage(`Failed to save template`);
      }

      setIsSaveTemplateDialogOpen(false);
      setNodeToSaveAsTemplate(null);
    },
    [nodeToSaveAsTemplate, workflow, terminal]
  );

  const handleCloseSaveTemplateDialog = useCallback(() => {
    setIsSaveTemplateDialogOpen(false);
    setNodeToSaveAsTemplate(null);
  }, []);

  const handleOpenWebhookSettings = useCallback(() => {
    setIsWebhookDialogOpen(true);
  }, []);

  const handleCloseWebhookSettings = useCallback(() => {
    setIsWebhookDialogOpen(false);
  }, []);

  const handleSaveWebhookSettings = useCallback(
    (
      webhookConfig: WebhookConfig | undefined,
      incomingConfig: IncomingWebhookConfig | undefined
    ) => {
      console.log('[WorkflowEditor] handleSaveWebhookSettings called');
      console.log('[WorkflowEditor] incomingConfig:', incomingConfig);
      console.log('[WorkflowEditor] incomingConfig?.token length:', incomingConfig?.token?.length);
      if (workflow) {
        const updatedWorkflow = {
          ...workflow,
          webhook: webhookConfig,
          incomingWebhook: incomingConfig,
        };
        console.log(
          '[WorkflowEditor] updatedWorkflow.incomingWebhook:',
          updatedWorkflow.incomingWebhook
        );
        updateWorkflow(updatedWorkflow);
      }
    },
    [workflow, updateWorkflow]
  );

  const isRunning = executionStatus === 'running';
  const canExecute = !isRunning && workflow !== null && workflow.nodes.length > 0;

  if (!workflow) {
    return (
      <div className="flex items-center justify-center h-full bg-background text-muted-foreground">
        Loading...
      </div>
    );
  }

  return (
    <ReactFlowProvider>
      <div className="flex flex-col h-full bg-background">
        <div className="shrink-0 border-b border-border">
          <WorkflowToolbar
            workflowName={workflow.name}
            executionStatus={executionStatus ?? undefined}
            canExecute={canExecute}
            isSaving={isSaving}
            saveSuccess={saveSuccess}
            onExecute={handleExecute}
            onCancel={handleCancel}
            onContinue={handleContinue}
            onNameChange={setName}
            onAddNode={handleAddNode}
            onAddTriggerWorkflow={handleAddTriggerWorkflow}
            onImportNode={handleImportNodeFromToolbar}
            onBack={onBack}
            onExportWorkflow={handleExportWorkflow}
            onImportWorkflow={handleImportWorkflow}
            onWebhookSettings={handleOpenWebhookSettings}
            hasOutgoingWebhook={!!workflow.webhook?.enabled}
            hasIncomingWebhook={!!workflow.incomingWebhook?.enabled}
            onHistory={() => setIsHistoryPanelOpen(true)}
            historyCount={historyCount}
          />
        </div>

        <div className="flex-1 flex overflow-hidden relative">
          <div
            className={cn('flex-1 transition-all duration-300', isNodePanelOpen && 'lg:mr-[400px]')}
          >
            <VisualCanvas
              nodes={workflow.nodes}
              nodeStatuses={nodeStatuses}
              childProgressMap={childProgressMap}
              selectedNodeId={selectedNodeId}
              disabled={isRunning}
              onSelectNode={handleSelectNode}
              onEditNode={handleEditNode}
              onDeleteNode={handleDeleteNode}
              onInsertNode={handleInsertNode}
              onInsertNodeBefore={handleInsertNodeBefore}
              onInsertNodeAfter={handleInsertNodeAfter}
              onDuplicateNode={handleDuplicateNode}
              onNodePositionChange={handleNodePositionChange}
              onReorderByPosition={reorderByPosition}
              onExportNode={handleExportNode}
              onSaveAsTemplate={handleSaveAsTemplate}
              onAddStep={handleAddNode}
              onFromTemplate={handleImportNodeFromToolbar}
            />
          </div>

          {selectedNode && selectedNode.type === 'trigger-workflow' ? (
            <TriggerWorkflowPanel
              node={selectedNode}
              status={nodeStatuses.get(selectedNode.id)}
              currentWorkflowId={workflow.id}
              isOpen={isNodePanelOpen}
              onClose={handleClosePanel}
              onSave={handleUpdateTriggerNodeFromPanel}
              onDelete={handleDeleteNode}
              disabled={isRunning}
            />
          ) : (
            <NodePanel
              node={selectedNode}
              status={selectedNode ? nodeStatuses.get(selectedNode.id) : undefined}
              isOpen={isNodePanelOpen}
              onClose={handleClosePanel}
              onSave={handleUpdateNodeFromPanel}
              onDelete={handleDeleteNode}
              disabled={isRunning}
            />
          )}
        </div>

        <div
          className={cn(
            'shrink-0 border-t border-border flex flex-col',
            isTerminalExpanded ? 'h-auto' : 'h-12',
            isResizing && 'select-none'
          )}
          style={isTerminalExpanded ? { height: localTerminalHeight } : undefined}
        >
          {isTerminalExpanded && (
            <div
              onMouseDown={handleResizeStart}
              className={cn(
                'h-1.5 cursor-ns-resize bg-secondary hover:bg-blue-500/50 transition-colors flex items-center justify-center group',
                isResizing && 'bg-blue-500/50'
              )}
            >
              <div className="w-10 h-0.5 bg-muted-foreground group-hover:bg-blue-400 rounded-full" />
            </div>
          )}
          <Button
            variant="ghost"
            onClick={handleToggleTerminal}
            className="w-full flex items-center justify-between px-4 py-3 bg-secondary hover:bg-accent transition-colors rounded-none h-auto"
          >
            <div className="flex items-center gap-2">
              <Terminal className="w-4 h-4 text-muted-foreground" />
              <span className="text-sm font-medium text-foreground">Output</span>
              {terminal.lines.length > 0 && (
                <span className="text-xs text-muted-foreground">
                  ({terminal.lines.length} lines)
                </span>
              )}
            </div>
            {isTerminalExpanded ? (
              <ChevronDown className="w-4 h-4 text-muted-foreground" />
            ) : (
              <ChevronUp className="w-4 h-4 text-muted-foreground" />
            )}
          </Button>

          {isTerminalExpanded && (
            <div className="flex-1 overflow-hidden min-h-0">
              <TerminalOutput lines={terminal.lines} className="h-full" />
            </div>
          )}
        </div>

        <NewNodeDialog
          isOpen={isNewNodeDialogOpen}
          defaultCwd={defaultCwd}
          insertIndex={insertPosition}
          onClose={handleCloseNewNodeDialog}
          onSave={handleSaveNewNode}
        />

        <SaveAsTemplateDialog
          isOpen={isSaveTemplateDialogOpen}
          node={
            nodeToSaveAsTemplate
              ? (workflow?.nodes.find((n) => n.id === nodeToSaveAsTemplate) ?? null)
              : null
          }
          onClose={handleCloseSaveTemplateDialog}
          onSave={handleSaveTemplateConfirm}
        />

        <WebhookSettingsDialog
          isOpen={isWebhookDialogOpen}
          workflowId={workflow.id}
          config={workflow.webhook}
          incomingConfig={workflow.incomingWebhook}
          onClose={handleCloseWebhookSettings}
          onSave={handleSaveWebhookSettings}
        />

        <ExecutionHistoryPanel
          workflowId={workflow.id}
          workflowName={workflow.name}
          isOpen={isHistoryPanelOpen}
          onClose={() => setIsHistoryPanelOpen(false)}
        />
      </div>
    </ReactFlowProvider>
  );
}
