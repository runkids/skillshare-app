import React, { useState, useRef, useEffect } from 'react';
import { Loader2, X, CheckCircle, XCircle, Play } from 'lucide-react';
import { Button } from '../ui/Button';
import { cn } from '../../lib/utils';
import {
  useBackgroundTasks,
  type BackgroundTask,
  type BackgroundTaskType,
} from '../../hooks/useBackgroundTasks';

// Task type icons
const getTaskIcon = (_type: BackgroundTaskType) => {
  return Play;
};

// Task type labels
const getTaskLabel = (_type: BackgroundTaskType) => {
  return 'Workflow';
};

// Format elapsed time
const formatElapsedTime = (startedAt: Date): string => {
  const now = new Date();
  const diffMs = now.getTime() - startedAt.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);

  if (diffSec < 60) return `${diffSec}s`;
  return `${diffMin}m ${diffSec % 60}s`;
};

interface TaskItemProps {
  task: BackgroundTask;
  isPanelOpen: boolean;
}

const TaskItem: React.FC<TaskItemProps> = ({ task, isPanelOpen }) => {
  const TaskIcon = getTaskIcon(task.type);
  const [elapsed, setElapsed] = useState(formatElapsedTime(task.startedAt));

  // Update elapsed time every second - ONLY when panel is open to save CPU/battery
  useEffect(() => {
    // Skip timer if panel is closed or task is not running
    if (!isPanelOpen || (task.status !== 'running' && task.status !== 'pending')) {
      return;
    }

    // Update immediately when panel opens
    setElapsed(formatElapsedTime(task.startedAt));

    const interval = setInterval(() => {
      setElapsed(formatElapsedTime(task.startedAt));
    }, 1000);

    return () => clearInterval(interval);
  }, [task.startedAt, task.status, isPanelOpen]);

  const isComplete = task.status === 'completed';
  const isFailed = task.status === 'failed';

  return (
    <div className="px-4 py-3 hover:bg-accent/50 transition-colors">
      <div className="flex items-start gap-3">
        {/* Icon with status indicator */}
        <div className="relative mt-0.5">
          <div
            className={cn(
              'p-1.5 rounded-lg',
              isComplete ? 'bg-green-500/20' : isFailed ? 'bg-red-500/20' : 'bg-blue-500/20'
            )}
          >
            <TaskIcon
              className={cn(
                'w-4 h-4',
                isComplete ? 'text-green-400' : isFailed ? 'text-red-400' : 'text-blue-400'
              )}
            />
          </div>
          {task.status === 'running' && (
            <span className="absolute -bottom-0.5 -right-0.5 w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-1.5">
              <span className="text-sm font-medium text-foreground">{task.name}</span>
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                {getTaskLabel(task.type)}
              </span>
            </div>
            {isComplete ? (
              <CheckCircle className="w-4 h-4 text-green-400 flex-shrink-0" />
            ) : isFailed ? (
              <XCircle className="w-4 h-4 text-red-400 flex-shrink-0" />
            ) : (
              <Loader2 className="w-4 h-4 text-blue-400 animate-spin flex-shrink-0" />
            )}
          </div>

          {/* Current step */}
          {task.currentStep && (
            <p className="text-xs text-muted-foreground mt-0.5 truncate">{task.currentStep}</p>
          )}

          {/* Progress bar */}
          {task.progress !== undefined && task.status === 'running' && (
            <div className="mt-2 h-1 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-blue-500 to-cyan-500 transition-all duration-300"
                style={{ width: `${task.progress}%` }}
              />
            </div>
          )}

          {/* Elapsed time */}
          <div className="flex items-center justify-between mt-1">
            <span className="text-[10px] text-muted-foreground/60">{elapsed}</span>
            {task.progress !== undefined && (
              <span className="text-[10px] text-muted-foreground/60">{task.progress}%</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export const BackgroundTasksButton: React.FC = () => {
  const [isOpen, setIsOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const { tasks, runningCount, isAnyRunning } = useBackgroundTasks();

  // Close panel on click outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        isOpen &&
        panelRef.current &&
        buttonRef.current &&
        !panelRef.current.contains(event.target as Node) &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen]);

  // Close panel on Escape
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isOpen) {
        setIsOpen(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  return (
    <div className="relative">
      {/* Button with spinner */}
      <Button
        ref={buttonRef}
        variant="ghost"
        size="icon"
        onClick={() => setIsOpen(!isOpen)}
        className="h-8 w-8 relative"
        aria-label="Background Tasks"
        aria-expanded={isOpen}
      >
        <Loader2
          className={cn(
            'w-4 h-4',
            isAnyRunning ? 'text-blue-400 animate-spin' : 'text-muted-foreground'
          )}
        />
        {runningCount > 0 && (
          <span className="absolute top-0 right-0 min-w-[14px] h-[14px] px-0.5 bg-gradient-to-r from-blue-500 to-cyan-500 text-white text-[10px] leading-[14px] rounded-full flex items-center justify-center shadow-sm border border-card">
            {runningCount > 9 ? '9+' : runningCount}
          </span>
        )}
      </Button>

      {/* Dropdown Panel */}
      {isOpen && (
        <div
          ref={panelRef}
          className={cn(
            'absolute right-0 top-full mt-2',
            'w-[340px] max-h-[400px]',
            'bg-card border border-border rounded-xl shadow-lg',
            'animate-in fade-in-0 zoom-in-95 slide-in-from-top-2 duration-150',
            'flex flex-col overflow-hidden',
            'z-50'
          )}
        >
          {/* Header */}
          <div className="px-4 py-3 border-b border-border flex items-center justify-between flex-shrink-0">
            <div className="flex items-center gap-2">
              <h3 className="font-medium text-sm text-foreground">Background Tasks</h3>
              {runningCount > 0 && (
                <span className="text-xs text-muted-foreground">({runningCount} running)</span>
              )}
            </div>
            <button
              onClick={() => setIsOpen(false)}
              className="p-1 hover:bg-accent rounded text-muted-foreground hover:text-foreground"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Task list */}
          <div className="flex-1 overflow-y-auto">
            {tasks.length === 0 ? (
              <div className="px-4 py-8 text-center">
                <CheckCircle className="w-8 h-8 text-green-400/30 mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">All tasks completed</p>
              </div>
            ) : (
              <div className="divide-y divide-border">
                {tasks.map((task) => (
                  <TaskItem key={task.id} task={task} isPanelOpen={isOpen} />
                ))}
              </div>
            )}
          </div>

          {/* Footer pointer */}
          <div className="absolute -top-1 right-4 w-2 h-2 bg-card border-l border-t border-border transform rotate-45" />
        </div>
      )}
    </div>
  );
};

export default BackgroundTasksButton;
