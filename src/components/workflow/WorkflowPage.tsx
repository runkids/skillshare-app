/**
 * Workflow Page Component
 * @see specs/001-expo-workflow-automation/spec.md - US3
 */

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Workflow as WorkflowIcon, Plus } from 'lucide-react';
import { Button } from '../ui/Button';
import { EmptyState } from '../ui/EmptyState';
import { WorkflowSidebar } from './WorkflowSidebar';
import { WorkflowEditor } from './WorkflowEditor';
import { useWorkflowExecutionContext } from '../../contexts/WorkflowExecutionContext';
import type { ExecutionStatus } from '../../types/workflow';
import {
  loadWorkflows,
  deleteWorkflow as deleteWorkflowApi,
  createWorkflow,
  saveWorkflow as saveWorkflowApi,
  generateId,
} from '../../lib/workflow-storage';
import { workflowAPI, settingsAPI, incomingWebhookAPI } from '../../lib/tauri-api';
import type { Workflow, RunningExecution } from '../../types/workflow';
import type { AppSettings, WorkflowSortMode } from '../../types/tauri';

interface WorkflowPageProps {
  initialWorkflow?: Workflow;
  defaultCwd?: string;
  onClearNavState?: () => void;
  dataVersion?: number;
}

export function WorkflowPage({
  initialWorkflow,
  defaultCwd,
  onClearNavState,
  dataVersion,
}: WorkflowPageProps) {
  const [workflows, setWorkflows] = useState<Workflow[]>([]);
  const [projects, setProjects] = useState<{ id: string; path: string }[]>([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState<Workflow | null>(null);
  const [currentDefaultCwd, setCurrentDefaultCwd] = useState<string | undefined>(defaultCwd);
  const [isLoading, setIsLoading] = useState(true);
  const [editorKey, setEditorKey] = useState(0);
  const [runningExecutions, setRunningExecutions] = useState<RunningExecution[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [workflowSortMode, setWorkflowSortMode] = useState<WorkflowSortMode>('updated');
  const [workflowOrder, setWorkflowOrder] = useState<string[]>([]);

  const initialWorkflowHandled = useRef(false);

  // Get execution states from context
  const { executions } = useWorkflowExecutionContext();

  // Create execution statuses map for sidebar
  const executionStatuses = useMemo(() => {
    const statuses = new Map<string, ExecutionStatus>();
    for (const [workflowId, state] of Object.entries(executions)) {
      if (state.status !== 'idle') {
        statuses.set(workflowId, state.status);
      }
    }
    return statuses;
  }, [executions]);

  // Helper: Get project path by projectId
  const getProjectPath = useCallback(
    (projectId: string | undefined): string | undefined => {
      if (!projectId) return undefined;
      const project = projects.find((p) => p.id === projectId);
      return project?.path;
    },
    [projects]
  );

  const fetchWorkflows = useCallback(async () => {
    try {
      const [loaded, loadedSettings] = await Promise.all([
        loadWorkflows(),
        settingsAPI.loadSettings(),
      ]);
      setWorkflows(loaded);
      setProjects([]);
      setSettings(loadedSettings);
      setWorkflowSortMode((loadedSettings.workflowSortMode as WorkflowSortMode) || 'updated');
      setWorkflowOrder(loadedSettings.workflowOrder || []);
    } catch (error) {
      console.error('Failed to load workflows:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const checkRunningExecutions = useCallback(async () => {
    try {
      await workflowAPI.restoreRunningExecutions();
      const executions = await workflowAPI.getRunningExecutions();
      const runningList: RunningExecution[] = Object.entries(executions)
        .filter(([_, exec]) => exec.status === 'running')
        .map(([id, exec]) => ({
          executionId: id,
          workflowId: exec.workflowId,
          workflowName: '', // Will be filled from workflows list if needed
          status: exec.status,
          currentNodeIndex: 0,
          pid: null,
          nodeId: null,
          command: null,
          cwd: null,
          startedAt: exec.startedAt,
          isRunning: true,
        }));
      setRunningExecutions(runningList);
    } catch (error) {
      console.error('Failed to check running executions:', error);
    }
  }, []);

  useEffect(() => {
    fetchWorkflows();
    checkRunningExecutions();
  }, [fetchWorkflows, checkRunningExecutions]);

  useEffect(() => {
    if (dataVersion !== undefined && dataVersion > 0) {
      fetchWorkflows();
    }
  }, [dataVersion, fetchWorkflows]);

  useEffect(() => {
    if (!isLoading && workflows.length > 0 && !selectedWorkflow && !initialWorkflow && settings) {
      const lastWorkflowId = settings.lastWorkflowId;
      if (lastWorkflowId) {
        const lastWorkflow = workflows.find((w) => w.id === lastWorkflowId);
        if (lastWorkflow) {
          setSelectedWorkflow(lastWorkflow);
          // If workflow is bound to a project, use project path as default cwd
          const projectPath = getProjectPath(lastWorkflow.projectId);
          setCurrentDefaultCwd(projectPath);
          setEditorKey((k) => k + 1);
        }
      }
    }
  }, [isLoading, workflows, selectedWorkflow, initialWorkflow, settings, getProjectPath]);

  const saveLastWorkflowId = useCallback(
    async (workflowId: string | null) => {
      if (!settings) return;
      const updatedSettings = { ...settings, lastWorkflowId: workflowId ?? undefined };
      setSettings(updatedSettings);
      try {
        await settingsAPI.saveSettings(updatedSettings);
      } catch (error) {
        console.error('Failed to save settings:', error);
      }
    },
    [settings]
  );

  useEffect(() => {
    if (initialWorkflow && !initialWorkflowHandled.current) {
      setSelectedWorkflow(initialWorkflow);
      setCurrentDefaultCwd(defaultCwd);
      setEditorKey((k) => k + 1);
      initialWorkflowHandled.current = true;
    }
  }, [initialWorkflow, defaultCwd]);

  useEffect(() => {
    return () => {
      if (onClearNavState) {
        onClearNavState();
      }
    };
  }, [onClearNavState]);

  const handleSelectWorkflow = useCallback(
    (workflow: Workflow) => {
      setSelectedWorkflow(workflow);
      // If workflow is bound to a project, use project path as default cwd
      const projectPath = getProjectPath(workflow.projectId);
      setCurrentDefaultCwd(projectPath);
      setEditorKey((k) => k + 1);
      saveLastWorkflowId(workflow.id);
    },
    [saveLastWorkflowId, getProjectPath]
  );

  const handleCreateWorkflow = useCallback(async () => {
    const newWorkflow = createWorkflow('New Workflow');

    const response = await saveWorkflowApi(newWorkflow);
    if (response.success && response.workflow) {
      setWorkflows((prev) => [...prev, response.workflow!]);
      setSelectedWorkflow(response.workflow);
      setEditorKey((k) => k + 1);
      saveLastWorkflowId(response.workflow.id);
    }
  }, [saveLastWorkflowId]);

  useEffect(() => {
    const handleShortcutNewWorkflow = () => {
      handleCreateWorkflow();
    };

    window.addEventListener('shortcut-new-workflow', handleShortcutNewWorkflow);
    return () => window.removeEventListener('shortcut-new-workflow', handleShortcutNewWorkflow);
  }, [handleCreateWorkflow]);

  const handleDeleteWorkflow = useCallback(
    async (workflowId: string) => {
      try {
        const response = await deleteWorkflowApi(workflowId);
        if (response.success) {
          setWorkflows((prev) => prev.filter((w) => w.id !== workflowId));
          setSelectedWorkflow((prev) => {
            if (prev?.id === workflowId) {
              saveLastWorkflowId(null);
              return null;
            }
            return prev;
          });
          if (settings?.lastWorkflowId === workflowId) {
            saveLastWorkflowId(null);
          }
        }
      } catch (error) {
        console.error('Failed to delete workflow:', error);
      }
    },
    [settings, saveLastWorkflowId]
  );

  const handleBack = useCallback(() => {
    setSelectedWorkflow(null);
    saveLastWorkflowId(null);
    fetchWorkflows();
  }, [fetchWorkflows, saveLastWorkflowId]);

  const handleDuplicateWorkflow = useCallback(
    async (workflow: Workflow) => {
      const duplicatedWorkflow = createWorkflow(`${workflow.name} (copy)`);
      duplicatedWorkflow.nodes = workflow.nodes.map((node) => ({
        ...JSON.parse(JSON.stringify(node)),
        id: generateId(),
      }));
      if (workflow.description) {
        duplicatedWorkflow.description = workflow.description;
      }
      if (workflow.projectId) {
        duplicatedWorkflow.projectId = workflow.projectId;
      }
      if (workflow.webhook) {
        duplicatedWorkflow.webhook = JSON.parse(JSON.stringify(workflow.webhook));
      }
      if (workflow.incomingWebhook) {
        try {
          const newConfig = await incomingWebhookAPI.createConfig();
          duplicatedWorkflow.incomingWebhook = {
            ...newConfig,
            enabled: false,
          };
        } catch (error) {
          console.error('Failed to create incoming webhook config for duplicate:', error);
        }
      }

      const response = await saveWorkflowApi(duplicatedWorkflow);
      if (response.success && response.workflow) {
        setWorkflows((prev) => [...prev, response.workflow!]);
        setSelectedWorkflow(response.workflow);
        setEditorKey((k) => k + 1);
        saveLastWorkflowId(response.workflow.id);
      }
    },
    [saveLastWorkflowId]
  );

  const handleKillProcess = useCallback(async (executionId: string) => {
    try {
      await workflowAPI.killProcess(executionId);
      setRunningExecutions((prev) => prev.filter((e) => e.executionId !== executionId));
    } catch (error) {
      console.error('Failed to kill process:', error);
    }
  }, []);

  const handleWorkflowSaved = useCallback(
    (savedWorkflow: Workflow) => {
      setWorkflows((prev) => {
        const index = prev.findIndex((w) => w.id === savedWorkflow.id);
        if (index !== -1) {
          const updated = [...prev];
          updated[index] = savedWorkflow;
          return updated;
        } else {
          return [...prev, savedWorkflow];
        }
      });
      setSelectedWorkflow(savedWorkflow);
      saveLastWorkflowId(savedWorkflow.id);
    },
    [saveLastWorkflowId]
  );

  const handleSortModeChange = useCallback(
    async (mode: WorkflowSortMode) => {
      setWorkflowSortMode(mode);
      if (settings) {
        const updatedSettings = { ...settings, workflowSortMode: mode };
        setSettings(updatedSettings);
        try {
          await settingsAPI.saveSettings(updatedSettings);
        } catch (error) {
          console.error('Failed to save sort mode:', error);
        }
      }
    },
    [settings]
  );

  const handleWorkflowOrderChange = useCallback(
    async (order: string[]) => {
      setWorkflowOrder(order);
      setWorkflowSortMode('custom');
      if (settings) {
        const updatedSettings = {
          ...settings,
          workflowSortMode: 'custom' as WorkflowSortMode,
          workflowOrder: order,
        };
        setSettings(updatedSettings);
        try {
          await settingsAPI.saveSettings(updatedSettings);
        } catch (error) {
          console.error('Failed to save workflow order:', error);
        }
      }
    },
    [settings]
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full bg-background text-muted-foreground">
        Loading...
      </div>
    );
  }

  return (
    <div className="flex h-full bg-background">
      <div className="w-60 shrink-0 border-r border-border">
        <WorkflowSidebar
          workflows={workflows}
          selectedWorkflowId={selectedWorkflow?.id ?? null}
          sortMode={workflowSortMode}
          workflowOrder={workflowOrder}
          executionStatuses={executionStatuses}
          onSelectWorkflow={handleSelectWorkflow}
          onCreateWorkflow={handleCreateWorkflow}
          onDeleteWorkflow={handleDeleteWorkflow}
          onDuplicateWorkflow={handleDuplicateWorkflow}
          onSortModeChange={handleSortModeChange}
          onWorkflowOrderChange={handleWorkflowOrderChange}
        />
      </div>

      <div className="flex-1">
        {selectedWorkflow ? (
          <WorkflowEditor
            key={editorKey}
            initialWorkflow={selectedWorkflow ?? undefined}
            defaultCwd={currentDefaultCwd}
            onBack={handleBack}
            onSaved={handleWorkflowSaved}
          />
        ) : (
          <div className="flex flex-col h-full bg-card">
            {runningExecutions.length > 0 && (
              <div className="p-4 border-b border-border">
                <h3 className="text-sm font-medium text-muted-foreground mb-3">
                  Running processes
                </h3>
                <div className="space-y-2">
                  {runningExecutions.map((exec) => (
                    <div
                      key={exec.executionId}
                      className="flex items-center justify-between p-3 bg-secondary rounded-lg border border-green-600/30"
                    >
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                          <span className="text-sm font-medium text-foreground truncate">
                            {exec.workflowName}
                          </span>
                        </div>
                        <div className="mt-1 text-xs text-muted-foreground font-mono truncate">
                          {exec.command}
                        </div>
                        {exec.pid && (
                          <div className="mt-1 text-xs text-muted-foreground">PID: {exec.pid}</div>
                        )}
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleKillProcess(exec.executionId)}
                        className="ml-3 h-auto px-3 py-1.5 text-xs bg-red-600/20 hover:bg-red-600/40 text-red-400"
                      >
                        Stop
                      </Button>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <EmptyState
              icon={WorkflowIcon}
              title="Select or Create a Workflow"
              description="Pick a workflow from the sidebar or create a new one to automate your tasks."
              variant="blue"
              showBackgroundPattern
              iconSize="lg"
              action={{
                label: 'Create Workflow',
                icon: Plus,
                onClick: handleCreateWorkflow,
              }}
              shortcuts={[{ key: '⌘N', label: 'New workflow' }]}
              className="flex-1"
            />
          </div>
        )}
      </div>
    </div>
  );
}
