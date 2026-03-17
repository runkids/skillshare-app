import { useState, useEffect, useCallback, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { NodeStartedPayload, ExecutionCompletedPayload } from '../lib/tauri-api';

// ============================================================================
// Types
// ============================================================================

export type BackgroundTaskType = 'workflow';
export type BackgroundTaskStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface BackgroundTask {
  id: string;
  type: BackgroundTaskType;
  name: string;
  status: BackgroundTaskStatus;
  progress?: number;
  currentStep?: string;
  startedAt: Date;
  completedAt?: Date;
  metadata?: {
    workflowId?: string;
    projectId?: string;
    projectName?: string;
  };
}

// Minimum time a task should be visible (ms)
const MIN_VISIBLE_TIME = 2000;
// Time to show completed status before removal (ms)
const COMPLETION_DISPLAY_TIME = 3000;

export interface UseBackgroundTasksReturn {
  tasks: BackgroundTask[];
  runningCount: number;
  isAnyRunning: boolean;
}

// ============================================================================
// Hook Implementation
// ============================================================================

export function useBackgroundTasks(): UseBackgroundTasksReturn {
  const [tasks, setTasks] = useState<BackgroundTask[]>([]);

  // Ref to track unlisten functions
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  // Helper to add or update a task
  const upsertTask = useCallback((taskId: string, updates: Partial<BackgroundTask>) => {
    setTasks((prev) => {
      const existingIndex = prev.findIndex((t) => t.id === taskId);
      if (existingIndex >= 0) {
        // Update existing
        const updated = [...prev];
        updated[existingIndex] = { ...updated[existingIndex], ...updates };
        return updated;
      } else {
        // Add new
        return [
          ...prev,
          {
            id: taskId,
            type: updates.type || 'workflow',
            name: updates.name || 'Unknown Task',
            status: updates.status || 'running',
            startedAt: updates.startedAt || new Date(),
            ...updates,
          } as BackgroundTask,
        ];
      }
    });
  }, []);

  // Helper to remove a task
  const removeTask = useCallback((taskId: string) => {
    setTasks((prev) => prev.filter((t) => t.id !== taskId));
  }, []);

  // Auto-remove completed tasks after delay, ensuring minimum visible time
  const scheduleRemoval = useCallback(
    (taskId: string) => {
      setTasks((prev) => {
        const task = prev.find((t) => t.id === taskId);
        if (!task) return prev;

        const now = Date.now();
        const elapsed = now - task.startedAt.getTime();
        const remainingMinTime = Math.max(0, MIN_VISIBLE_TIME - elapsed);
        const totalDelay = remainingMinTime + COMPLETION_DISPLAY_TIME;

        setTimeout(() => {
          removeTask(taskId);
        }, totalDelay);

        // Update completedAt
        return prev.map((t) => (t.id === taskId ? { ...t, completedAt: new Date() } : t));
      });
    },
    [removeTask]
  );

  // Setup event listeners
  useEffect(() => {
    const setupListeners = async () => {
      // Workflow events
      const unlistenWorkflowStart = await listen<NodeStartedPayload>(
        'execution_node_started',
        (event) => {
          const { executionId, nodeName, targetWorkflowName } =
            event.payload as NodeStartedPayload & { executionId?: string };
          if (executionId) {
            upsertTask(executionId, {
              type: 'workflow',
              name: targetWorkflowName || nodeName || 'Workflow',
              status: 'running',
              currentStep: nodeName,
            });
          }
        }
      );

      const unlistenWorkflowComplete = await listen<ExecutionCompletedPayload>(
        'execution_completed',
        (event) => {
          const { executionId, status } = event.payload as ExecutionCompletedPayload & {
            executionId?: string;
          };
          if (executionId) {
            upsertTask(executionId, {
              status: status === 'completed' ? 'completed' : 'failed',
            });
            scheduleRemoval(executionId);
          }
        }
      );

      unlistenRefs.current = [unlistenWorkflowStart, unlistenWorkflowComplete];
    };

    setupListeners();

    return () => {
      unlistenRefs.current.forEach((unlisten) => unlisten());
      unlistenRefs.current = [];
    };
  }, [upsertTask, scheduleRemoval]);

  // Computed values
  const runningTasks = tasks.filter((t) => t.status === 'running' || t.status === 'pending');
  const runningCount = runningTasks.length;
  const isAnyRunning = runningCount > 0;

  return {
    tasks: runningTasks,
    runningCount,
    isAnyRunning,
  };
}

export default useBackgroundTasks;
