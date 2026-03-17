// Tauri API Wrapper
// Provides a unified interface for frontend to communicate with Rust backend

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppSettings, StoreData, StorePathInfo } from '../types/tauri';
import type { Workflow } from '../types';

// Re-export plugin APIs
export { open, save, message, confirm } from '@tauri-apps/plugin-dialog';
export { openUrl } from '@tauri-apps/plugin-opener';
export { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs';

// Re-export for use in other modules
export { invoke, listen };
export type { UnlistenFn };

// ============================================================================
// Script Commands (Phase 6 - US3)
// ============================================================================

// Script execution types
export interface ExecuteScriptParams {
  projectPath: string;
  scriptName: string;
  packageManager: string;
  cwd?: string;
}

export interface ExecuteCommandParams {
  command: string;
  args: string[];
  cwd: string;
}

export interface ExecuteScriptResponse {
  success: boolean;
  executionId?: string;
  error?: string;
}

export interface CancelScriptResponse {
  success: boolean;
  error?: string;
}

export interface ScriptOutputPayload {
  executionId: string;
  output: string;
  stream: 'stdout' | 'stderr';
  timestamp: string;
}

export interface ScriptCompletedPayload {
  executionId: string;
  exitCode: number;
  success: boolean;
  durationMs: number;
}

export type ScriptExecutionStatus = 'running' | 'completed' | 'failed' | 'cancelled';

export interface WriteToScriptResponse {
  success: boolean;
  error?: string;
}

export interface RunningScriptInfo {
  executionId: string;
  scriptName: string;
  startedAtMs: number;
  projectPath: string;
  projectName?: string;
  startedAt: string;
  status: ScriptExecutionStatus;
  exitCode?: number;
  completedAt?: string;
}

export interface OutputLine {
  content: string;
  stream: 'stdout' | 'stderr';
  timestamp: string;
}

export interface GetScriptOutputResponse {
  success: boolean;
  executionId: string;
  output?: string;
  lines?: OutputLine[];
  truncated: boolean;
  bufferSize: number;
  error?: string;
}

export const scriptAPI = {
  executeScript: (params: ExecuteScriptParams): Promise<ExecuteScriptResponse> =>
    invoke<ExecuteScriptResponse>('execute_script', { ...params }),

  executeCommand: (params: ExecuteCommandParams): Promise<ExecuteScriptResponse> =>
    invoke<ExecuteScriptResponse>('execute_command', { ...params }),

  cancelScript: (executionId: string): Promise<CancelScriptResponse> =>
    invoke<CancelScriptResponse>('cancel_script', { executionId }),

  killAllNodeProcesses: (): Promise<CancelScriptResponse> =>
    invoke<CancelScriptResponse>('kill_all_node_processes'),

  killPorts: (ports: number[]): Promise<CancelScriptResponse> =>
    invoke<CancelScriptResponse>('kill_ports', { ports }),

  checkPorts: (ports: number[]): Promise<number[]> => invoke<number[]>('check_ports', { ports }),

  getRunningScripts: (): Promise<RunningScriptInfo[]> =>
    invoke<RunningScriptInfo[]>('get_running_scripts'),

  getScriptOutput: (executionId: string): Promise<GetScriptOutputResponse> =>
    invoke<GetScriptOutputResponse>('get_script_output', { executionId }),

  writeToScript: (executionId: string, input: string): Promise<WriteToScriptResponse> =>
    invoke<WriteToScriptResponse>('write_to_script', { executionId, input }),

  getPtyEnv: (): Promise<Record<string, string>> => invoke<Record<string, string>>('get_pty_env'),

  getVoltaWrappedCommand: (
    command: string,
    args: string[],
    cwd: string
  ): Promise<VoltaWrappedCommand> =>
    invoke<VoltaWrappedCommand>('get_volta_wrapped_command', { command, args, cwd }),
};

export interface VoltaWrappedCommand {
  command: string;
  args: string[];
  useVolta: boolean;
}

// ============================================================================
// Workflow Commands (Phase 7 - US4)
// ============================================================================

export interface NodeStartedPayload {
  executionId: string;
  workflowId: string;
  nodeId: string;
  nodeName: string;
  nodeType: 'script' | 'trigger-workflow';
  targetWorkflowName?: string;
  startedAt: string;
}

export interface ExecutionOutputPayload {
  executionId: string;
  workflowId: string;
  nodeId: string;
  output: string;
  stream: 'stdout' | 'stderr';
  timestamp: string;
}

export interface NodeCompletedPayload {
  executionId: string;
  workflowId: string;
  nodeId: string;
  status: 'completed' | 'failed' | 'cancelled';
  exitCode?: number;
  errorMessage?: string;
  finishedAt: string;
}

export interface ExecutionCompletedPayload {
  executionId: string;
  workflowId: string;
  status: 'completed' | 'failed' | 'cancelled';
  finishedAt: string;
  totalDurationMs: number;
}

export interface ExecutionPausedPayload {
  executionId: string;
  workflowId: string;
  pausedAtNodeId: string;
  reason: string;
}

export interface Execution {
  id: string;
  workflowId: string;
  status: 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';
  startedAt: string;
  finishedAt?: string;
  nodeResults: Record<
    string,
    {
      nodeId: string;
      status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
      startedAt?: string;
      finishedAt?: string;
      output?: string;
      exitCode?: number;
      errorMessage?: string;
    }
  >;
}

export interface AvailableWorkflowInfo {
  id: string;
  name: string;
  description?: string;
  stepCount: number;
  projectId?: string;
  projectName?: string;
  lastExecutedAt?: string;
}

export interface CycleDetectionResult {
  hasCycle: boolean;
  cyclePath?: string[];
  cycleDescription?: string;
}

export interface ChildExecutionInfo {
  executionId: string;
  workflowId: string;
  workflowName: string;
  status: string;
  startedAt?: string;
  finishedAt?: string;
  depth: number;
}

export interface WorkflowOutputLine {
  nodeId: string;
  nodeName: string;
  content: string;
  stream: 'stdout' | 'stderr' | 'system';
  timestamp: string;
}

export interface WorkflowOutputResponse {
  found: boolean;
  workflowId?: string;
  executionId?: string;
  lines: WorkflowOutputLine[];
  truncated: boolean;
  bufferSize: number;
}

export interface ExecutionHistoryItem {
  id: string;
  workflowId: string;
  workflowName: string;
  status: string;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  nodeCount: number;
  completedNodeCount: number;
  errorMessage?: string;
  output: WorkflowOutputLine[];
  triggeredBy: string;
}

export interface ExecutionHistorySettings {
  maxHistoryPerWorkflow: number;
  retentionDays: number;
  maxOutputLines: number;
}

export interface ExecutionHistoryStoreData {
  version: string;
  histories: Record<string, ExecutionHistoryItem[]>;
  settings?: ExecutionHistorySettings;
}

export const workflowAPI = {
  saveWorkflow: (workflow: Workflow): Promise<void> => invoke('save_workflow', { workflow }),

  deleteWorkflow: (workflowId: string): Promise<void> => invoke('delete_workflow', { workflowId }),

  executeWorkflow: (workflowId: string): Promise<string> =>
    invoke<string>('execute_workflow', { workflowId }),

  cancelExecution: (executionId: string): Promise<void> =>
    invoke('cancel_execution', { executionId }),

  continueExecution: (executionId: string): Promise<void> =>
    invoke('continue_execution', { executionId }),

  getRunningExecutions: (): Promise<Record<string, Execution>> =>
    invoke<Record<string, Execution>>('get_running_executions'),

  restoreRunningExecutions: (): Promise<void> => invoke('restore_running_executions'),

  killProcess: (executionId: string): Promise<void> => invoke('kill_process', { executionId }),

  getAvailableWorkflows: (excludeWorkflowId: string): Promise<AvailableWorkflowInfo[]> =>
    invoke<AvailableWorkflowInfo[]>('get_available_workflows', { excludeWorkflowId }),

  detectWorkflowCycle: (
    sourceWorkflowId: string,
    targetWorkflowId: string
  ): Promise<CycleDetectionResult> =>
    invoke<CycleDetectionResult>('detect_workflow_cycle', { sourceWorkflowId, targetWorkflowId }),

  getChildExecutions: (parentExecutionId: string): Promise<ChildExecutionInfo[]> =>
    invoke<ChildExecutionInfo[]>('get_child_executions', { parentExecutionId }),

  getWorkflowOutput: (workflowId: string): Promise<WorkflowOutputResponse> =>
    invoke<WorkflowOutputResponse>('get_workflow_output', { workflowId }),

  loadExecutionHistory: (workflowId: string): Promise<ExecutionHistoryItem[]> =>
    invoke<ExecutionHistoryItem[]>('load_execution_history', { workflowId }),

  loadAllExecutionHistory: (): Promise<ExecutionHistoryStoreData> =>
    invoke<ExecutionHistoryStoreData>('load_all_execution_history'),

  saveExecutionHistory: (item: ExecutionHistoryItem): Promise<void> =>
    invoke('save_execution_history', { item }),

  deleteExecutionHistory: (workflowId: string, historyId: string): Promise<void> =>
    invoke('delete_execution_history', { workflowId, historyId }),

  clearWorkflowExecutionHistory: (workflowId: string): Promise<void> =>
    invoke('clear_workflow_execution_history', { workflowId }),

  updateExecutionHistorySettings: (settings: ExecutionHistorySettings): Promise<void> =>
    invoke('update_execution_history_settings', { settings }),
};

// ============================================================================
// Webhook API (Feature 012 - Workflow Webhook Support)
// ============================================================================

import type { WebhookDeliveryEvent, WebhookTestResult } from '../types/webhook';

const webhookUnlisteners: UnlistenFn[] = [];

export const webhookAPI = {
  onWebhookDelivery: (callback: (event: WebhookDeliveryEvent) => void): void => {
    listen<WebhookDeliveryEvent>('webhook_delivery', (event) => {
      callback(event.payload);
    }).then((unlisten) => {
      webhookUnlisteners.push(unlisten);
    });
  },

  removeWebhookListeners: (): void => {
    webhookUnlisteners.forEach((unlisten) => unlisten());
    webhookUnlisteners.length = 0;
  },

  testWebhook: (
    url: string,
    headers?: Record<string, string>,
    payloadTemplate?: string
  ): Promise<WebhookTestResult> =>
    invoke<WebhookTestResult>('test_webhook', { url, headers, payloadTemplate }),
};

// ============================================================================
// Worktree Commands (Phase 8 - US5)
// ============================================================================

export interface Worktree {
  path: string;
  branch: string | null;
  head: string;
  isMain: boolean;
  isBare?: boolean;
  isDetached?: boolean;
}

export interface WorktreeStatus {
  uncommittedCount: number;
  ahead: number;
  behind: number;
  hasTrackingBranch: boolean;
  lastCommitTime: string | null;
  lastCommitMessage: string | null;
  hasRunningProcess: boolean;
}

export interface EditorDefinition {
  id: string;
  name: string;
  command: string;
  args: string[];
  isAvailable: boolean;
}

export interface GetWorktreeStatusResponse {
  success: boolean;
  status?: WorktreeStatus;
  error?: string;
}

export interface GetAllWorktreeStatusesResponse {
  success: boolean;
  statuses?: Record<string, WorktreeStatus>;
  error?: string;
}

export interface OpenInEditorResponse {
  success: boolean;
  editor?: string;
  error?: string;
}

export interface GetAvailableEditorsResponse {
  success: boolean;
  editors?: EditorDefinition[];
  defaultEditor?: string;
  error?: string;
}

export interface ExecuteScriptInWorktreeResponse {
  success: boolean;
  executionId?: string;
  error?: string;
}

export interface ListWorktreesResponse {
  success: boolean;
  worktrees?: Worktree[];
  error?: string;
}

export interface ListBranchesResponse {
  success: boolean;
  branches?: string[];
  error?: string;
}

export interface AddWorktreeResponse {
  success: boolean;
  worktree?: Worktree;
  error?: string;
}

export interface RemoveWorktreeResponse {
  success: boolean;
  error?: string;
}

export interface GetMergedWorktreesResponse {
  success: boolean;
  mergedWorktrees?: Worktree[];
  baseBranch?: string;
  error?: string;
}

export interface CommitInfo {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  date: string;
}

export interface GetBehindCommitsResponse {
  success: boolean;
  behindCount: number;
  commits?: CommitInfo[];
  baseBranch?: string;
  error?: string;
}

export interface SyncWorktreeResponse {
  success: boolean;
  method?: string;
  hasConflicts: boolean;
  error?: string;
}

export interface AddWorktreeParams {
  projectPath: string;
  worktreePath: string;
  branch: string;
  createBranch: boolean;
}

export interface RemoveWorktreeParams {
  projectPath: string;
  worktreePath: string;
  force?: boolean;
  deleteBranch?: boolean;
}

export const worktreeAPI = {
  isGitRepo: (projectPath: string): Promise<boolean> =>
    invoke<boolean>('is_git_repo', { projectPath }),

  listBranches: (projectPath: string): Promise<ListBranchesResponse> =>
    invoke<ListBranchesResponse>('list_branches', { projectPath }),

  listWorktrees: (projectPath: string): Promise<ListWorktreesResponse> =>
    invoke<ListWorktreesResponse>('list_worktrees', { projectPath }),

  addWorktree: (params: AddWorktreeParams): Promise<AddWorktreeResponse> =>
    invoke<AddWorktreeResponse>('add_worktree', { ...params }),

  removeWorktree: (params: RemoveWorktreeParams): Promise<RemoveWorktreeResponse> =>
    invoke<RemoveWorktreeResponse>('remove_worktree', {
      projectPath: params.projectPath,
      worktreePath: params.worktreePath,
      force: params.force ?? false,
      deleteBranch: params.deleteBranch ?? false,
    }),

  getMergedWorktrees: (
    projectPath: string,
    baseBranch?: string
  ): Promise<GetMergedWorktreesResponse> =>
    invoke<GetMergedWorktreesResponse>('get_merged_worktrees', { projectPath, baseBranch }),

  getBehindCommits: (
    worktreePath: string,
    baseBranch?: string,
    limit?: number
  ): Promise<GetBehindCommitsResponse> =>
    invoke<GetBehindCommitsResponse>('get_behind_commits', { worktreePath, baseBranch, limit }),

  syncWorktree: (
    worktreePath: string,
    baseBranch: string,
    method: 'rebase' | 'merge'
  ): Promise<SyncWorktreeResponse> =>
    invoke<SyncWorktreeResponse>('sync_worktree', { worktreePath, baseBranch, method }),

  getWorktreeStatus: (worktreePath: string): Promise<GetWorktreeStatusResponse> =>
    invoke<GetWorktreeStatusResponse>('get_worktree_status', { worktreePath }),

  getAllWorktreeStatuses: (projectPath: string): Promise<GetAllWorktreeStatusesResponse> =>
    invoke<GetAllWorktreeStatusesResponse>('get_all_worktree_statuses', { projectPath }),

  executeScriptInWorktree: (params: {
    worktreePath: string;
    scriptName: string;
    packageManager: string;
  }): Promise<ExecuteScriptInWorktreeResponse> =>
    invoke<ExecuteScriptInWorktreeResponse>('execute_script_in_worktree', { ...params }),

  openInEditor: (worktreePath: string, editorId?: string): Promise<OpenInEditorResponse> =>
    invoke<OpenInEditorResponse>('open_in_editor', { worktreePath, editorId }),

  getAvailableEditors: (): Promise<GetAvailableEditorsResponse> =>
    invoke<GetAvailableEditorsResponse>('get_available_editors'),

  checkGitignoreHasWorktrees: (projectPath: string): Promise<CheckGitignoreResponse> =>
    invoke<CheckGitignoreResponse>('check_gitignore_has_worktrees', { projectPath }),

  addWorktreesToGitignore: (projectPath: string): Promise<AddToGitignoreResponse> =>
    invoke<AddToGitignoreResponse>('add_worktrees_to_gitignore', { projectPath }),
};

// ============================================================================
// Terminal Types & API
// ============================================================================

export interface TerminalDefinition {
  id: string;
  name: string;
  command?: string;
  bundleId?: string;
  args: string[];
  isAvailable: boolean;
  isBuiltin: boolean;
}

export interface GetAvailableTerminalsResponse {
  success: boolean;
  terminals?: TerminalDefinition[];
  defaultTerminal?: string;
  error?: string;
}

export interface OpenInTerminalResponse {
  success: boolean;
  terminal?: string;
  error?: string;
}

export const terminalAPI = {
  getAvailableTerminals: (): Promise<GetAvailableTerminalsResponse> =>
    invoke<GetAvailableTerminalsResponse>('get_available_terminals'),

  setPreferredTerminal: (terminalId: string): Promise<boolean> =>
    invoke<boolean>('set_preferred_terminal', { terminalId }),

  openInTerminal: (path: string, terminalId?: string): Promise<OpenInTerminalResponse> =>
    invoke<OpenInTerminalResponse>('open_in_terminal', { path, terminalId }),
};

// ============================================================================
// Gitignore Management Types
// ============================================================================

export interface CheckGitignoreResponse {
  success: boolean;
  hasWorktreesEntry: boolean;
  gitignoreExists: boolean;
  error?: string;
}

export interface AddToGitignoreResponse {
  success: boolean;
  createdFile: boolean;
  error?: string;
}

// ============================================================================
// Worktree Template Types & API
// ============================================================================

export interface WorktreeTemplate {
  id: string;
  name: string;
  description?: string;
  branchPattern: string;
  pathPattern: string;
  postCreateScripts: string[];
  openInEditor: boolean;
  preferredEditor?: string;
  baseBranch?: string;
  isDefault: boolean;
  createdAt: string;
  updatedAt?: string;
}

export interface CreateWorktreeFromTemplateParams {
  projectPath: string;
  templateId: string;
  name: string;
  customBaseBranch?: string;
}

export interface SaveTemplateResponse {
  success: boolean;
  template?: WorktreeTemplate;
  error?: string;
}

export interface DeleteTemplateResponse {
  success: boolean;
  error?: string;
}

export interface ListTemplatesResponse {
  success: boolean;
  templates?: WorktreeTemplate[];
  error?: string;
}

export interface GetNextFeatureNumberResponse {
  success: boolean;
  featureNumber?: string;
  error?: string;
}

export interface CreateWorktreeFromTemplateResponse {
  success: boolean;
  worktree?: Worktree;
  executedScripts?: string[];
  specFilePath?: string;
  error?: string;
}

export const worktreeTemplateAPI = {
  saveTemplate: (template: WorktreeTemplate): Promise<SaveTemplateResponse> =>
    invoke<SaveTemplateResponse>('save_worktree_template', { template }),

  deleteTemplate: (templateId: string): Promise<DeleteTemplateResponse> =>
    invoke<DeleteTemplateResponse>('delete_worktree_template', { templateId }),

  listTemplates: (): Promise<ListTemplatesResponse> =>
    invoke<ListTemplatesResponse>('list_worktree_templates'),

  getDefaultTemplates: (): Promise<ListTemplatesResponse> =>
    invoke<ListTemplatesResponse>('get_default_worktree_templates'),

  getNextFeatureNumber: (projectPath: string): Promise<GetNextFeatureNumberResponse> =>
    invoke<GetNextFeatureNumberResponse>('get_next_feature_number', { projectPath }),

  createWorktreeFromTemplate: (
    params: CreateWorktreeFromTemplateParams
  ): Promise<CreateWorktreeFromTemplateResponse> =>
    invoke<CreateWorktreeFromTemplateResponse>('create_worktree_from_template', { ...params }),
};

// ============================================================================
// IPA Commands
// ============================================================================

export interface IpaMetadata {
  fileName: string;
  filePath: string;
  bundleId: string;
  version: string;
  build: string;
  displayName: string;
  deviceCapabilities: string;
  error?: string;
  fullPlist?: Record<string, unknown>;
  createdAt: string;
}

export interface CheckHasIpaFilesResponse {
  success: boolean;
  hasIpaFiles: boolean;
  count: number;
  error?: string;
}

export interface ScanProjectIpaResponse {
  success: boolean;
  results: IpaMetadata[];
  error?: string;
}

export const ipaAPI = {
  checkHasIpaFiles: (dirPath: string): Promise<CheckHasIpaFilesResponse> =>
    invoke<CheckHasIpaFilesResponse>('check_has_ipa_files', { dirPath }),

  scanProjectIpa: (dirPath: string): Promise<ScanProjectIpaResponse> =>
    invoke<ScanProjectIpaResponse>('scan_project_ipa', { dirPath }),
};

// ============================================================================
// APK Commands
// ============================================================================

export interface ApkMetadata {
  fileName: string;
  filePath: string;
  packageName: string;
  versionName: string;
  versionCode: string;
  appName: string;
  minSdk: string;
  targetSdk: string;
  error?: string;
  createdAt: string;
  fileSize: number;
}

export interface CheckHasApkFilesResponse {
  success: boolean;
  hasApkFiles: boolean;
  count: number;
  error?: string;
}

export interface ScanProjectApkResponse {
  success: boolean;
  results: ApkMetadata[];
  error?: string;
}

export const apkAPI = {
  checkHasApkFiles: (dirPath: string): Promise<CheckHasApkFilesResponse> =>
    invoke<CheckHasApkFilesResponse>('check_has_apk_files', { dirPath }),

  scanProjectApk: (dirPath: string): Promise<ScanProjectApkResponse> =>
    invoke<ScanProjectApkResponse>('scan_project_apk', { dirPath }),
};

// ============================================================================
// Settings Commands
// ============================================================================

export const settingsAPI = {
  loadSettings: (): Promise<AppSettings> => invoke<AppSettings>('load_settings'),

  saveSettings: (settings: AppSettings): Promise<void> => invoke('save_settings', { settings }),

  loadWorkflows: (): Promise<Workflow[]> => invoke<Workflow[]>('load_workflows'),

  saveWorkflows: (workflows: Workflow[]): Promise<void> => invoke('save_workflows', { workflows }),

  loadStoreData: (): Promise<StoreData> => invoke<StoreData>('load_store_data'),

  getStorePath: (): Promise<StorePathInfo> => invoke<StorePathInfo>('get_store_path'),

  setStorePath: (newPath: string): Promise<StorePathInfo> =>
    invoke<StorePathInfo>('set_store_path', { newPath }),

  resetStorePath: (): Promise<StorePathInfo> => invoke<StorePathInfo>('reset_store_path'),

  openStoreLocation: (): Promise<void> => invoke('open_store_location'),
};

// ============================================================================
// Notification Settings Commands
// ============================================================================

import type {
  NotificationSettings,
  NotificationRecord,
  NotificationListResponse,
} from '../types/notification';

export type { NotificationSettings, NotificationRecord, NotificationListResponse };

export const notificationAPI = {
  loadSettings: (): Promise<NotificationSettings> =>
    invoke<NotificationSettings>('load_notification_settings'),

  saveSettings: (settings: NotificationSettings): Promise<void> =>
    invoke('save_notification_settings', { settings }),
};

// ============================================================================
// Notification History API (Notification Center)
// ============================================================================

export const notificationHistoryAPI = {
  getNotifications: (limit?: number, offset?: number): Promise<NotificationListResponse> =>
    invoke<NotificationListResponse>('get_notifications', { limit, offset }),

  getUnreadCount: (): Promise<number> => invoke<number>('get_unread_notification_count'),

  markAsRead: (id: string): Promise<boolean> => invoke<boolean>('mark_notification_read', { id }),

  markAllAsRead: (): Promise<number> => invoke<number>('mark_all_notifications_read'),

  deleteNotification: (id: string): Promise<boolean> =>
    invoke<boolean>('delete_notification', { id }),

  cleanupOld: (retentionDays?: number): Promise<number> =>
    invoke<number>('cleanup_old_notifications', { retentionDays }),

  clearAll: (): Promise<number> => invoke<number>('clear_all_notifications'),
};

// ============================================================================
// Event Listeners
// ============================================================================

export interface ChildExecutionStartedPayload {
  parentExecutionId: string;
  parentNodeId: string;
  childExecutionId: string;
  childWorkflowId: string;
  childWorkflowName: string;
  startedAt: string;
}

export interface ChildExecutionProgressPayload {
  parentExecutionId: string;
  parentNodeId: string;
  childExecutionId: string;
  currentStep: number;
  totalSteps: number;
  currentNodeId: string;
  currentNodeName: string;
  timestamp: string;
}

export interface ChildExecutionCompletedPayload {
  parentExecutionId: string;
  parentNodeId: string;
  childExecutionId: string;
  childWorkflowId: string;
  status: 'completed' | 'failed' | 'cancelled';
  durationMs: number;
  errorMessage?: string;
  finishedAt: string;
}

export const tauriEvents = {
  // Script events
  onScriptOutput: (callback: (data: ScriptOutputPayload) => void): Promise<UnlistenFn> =>
    listen<ScriptOutputPayload>('script_output', (event) => callback(event.payload)),

  onScriptCompleted: (callback: (data: ScriptCompletedPayload) => void): Promise<UnlistenFn> =>
    listen<ScriptCompletedPayload>('script_completed', (event) => callback(event.payload)),

  // Workflow events
  onWorkflowNodeStarted: (callback: (data: NodeStartedPayload) => void): Promise<UnlistenFn> =>
    listen<NodeStartedPayload>('execution_node_started', (event) => callback(event.payload)),

  onWorkflowOutput: (callback: (data: ExecutionOutputPayload) => void): Promise<UnlistenFn> =>
    listen<ExecutionOutputPayload>('execution_output', (event) => callback(event.payload)),

  onWorkflowNodeCompleted: (callback: (data: NodeCompletedPayload) => void): Promise<UnlistenFn> =>
    listen<NodeCompletedPayload>('execution_node_completed', (event) => callback(event.payload)),

  onWorkflowCompleted: (callback: (data: ExecutionCompletedPayload) => void): Promise<UnlistenFn> =>
    listen<ExecutionCompletedPayload>('execution_completed', (event) => callback(event.payload)),

  onWorkflowPaused: (callback: (data: ExecutionPausedPayload) => void): Promise<UnlistenFn> =>
    listen<ExecutionPausedPayload>('execution_paused', (event) => callback(event.payload)),

  // Child execution events
  onChildExecutionStarted: (
    callback: (data: ChildExecutionStartedPayload) => void
  ): Promise<UnlistenFn> =>
    listen<ChildExecutionStartedPayload>('child_execution_started', (event) =>
      callback(event.payload)
    ),

  onChildExecutionProgress: (
    callback: (data: ChildExecutionProgressPayload) => void
  ): Promise<UnlistenFn> =>
    listen<ChildExecutionProgressPayload>('child_execution_progress', (event) =>
      callback(event.payload)
    ),

  onChildExecutionCompleted: (
    callback: (data: ChildExecutionCompletedPayload) => void
  ): Promise<UnlistenFn> =>
    listen<ChildExecutionCompletedPayload>('child_execution_completed', (event) =>
      callback(event.payload)
    ),

  // File watcher events
  onPackageJsonChanged: (
    callback: (data: PackageJsonChangedPayload) => void
  ): Promise<UnlistenFn> =>
    listen<PackageJsonChangedPayload>('package-json-changed', (event) => callback(event.payload)),

  // Notification Center events
  onNewNotification: (callback: (data: NotificationRecord) => void): Promise<UnlistenFn> =>
    listen<NotificationRecord>('notification:new', (event) => callback(event.payload)),
};

// ============================================================================
// File Watcher Types and API
// ============================================================================

export interface PackageJsonChangedPayload {
  project_path: string;
  file_path: string;
}

export interface FileWatcherResponse {
  success: boolean;
  error?: string;
}

export const fileWatcherAPI = {
  watchProject: (projectPath: string): Promise<FileWatcherResponse> =>
    invoke<FileWatcherResponse>('watch_project', { projectPath }),

  unwatchProject: (projectPath: string): Promise<FileWatcherResponse> =>
    invoke<FileWatcherResponse>('unwatch_project', { projectPath }),

  unwatchAllProjects: (): Promise<FileWatcherResponse> =>
    invoke<FileWatcherResponse>('unwatch_all_projects'),

  getWatchedProjects: (): Promise<string[]> => invoke<string[]>('get_watched_projects'),
};

// ============================================================================
// Git Commands (009-git-integration)
// ============================================================================

import type {
  GitStatus,
  GitFile,
  Branch,
  Commit,
  Stash,
  GetGitStatusResponse,
  StageFilesResponse,
  UnstageFilesResponse,
  CreateCommitResponse,
  GetBranchesResponse,
  CreateBranchResponse,
  SwitchBranchResponse,
  DeleteBranchResponse,
  GetCommitHistoryResponse,
  GitPushResponse,
  GitPullResponse,
  ListStashesResponse,
  CreateStashResponse,
  ApplyStashResponse,
  DropStashResponse,
  GitRemote,
  GetRemotesResponse,
  AddRemoteResponse,
  RemoveRemoteResponse,
  DiscardChangesResponse,
  GitFetchResponse,
  GitRebaseResponse,
  GitAuthStatus,
  GetGitAuthStatusResponse,
  TestRemoteConnectionResponse,
  FileDiff,
  DiffHunk,
  DiffLine,
  GetFileDiffResponse,
} from '../types/git';

export type {
  GitStatus,
  GitFile,
  Branch,
  Commit,
  Stash,
  GetGitStatusResponse,
  StageFilesResponse,
  UnstageFilesResponse,
  CreateCommitResponse,
  GetBranchesResponse,
  CreateBranchResponse,
  SwitchBranchResponse,
  DeleteBranchResponse,
  GetCommitHistoryResponse,
  GitPushResponse,
  GitPullResponse,
  ListStashesResponse,
  CreateStashResponse,
  ApplyStashResponse,
  DropStashResponse,
  GitRemote,
  GetRemotesResponse,
  AddRemoteResponse,
  RemoveRemoteResponse,
  DiscardChangesResponse,
  GitFetchResponse,
  GitRebaseResponse,
  GitAuthStatus,
  GetGitAuthStatusResponse,
  TestRemoteConnectionResponse,
  FileDiff,
  DiffHunk,
  DiffLine,
  GetFileDiffResponse,
};

export const gitAPI = {
  getStatus: (projectPath: string): Promise<GetGitStatusResponse> =>
    invoke<GetGitStatusResponse>('get_git_status', { projectPath }),

  stageFiles: (projectPath: string, files: string[]): Promise<StageFilesResponse> =>
    invoke<StageFilesResponse>('stage_files', { projectPath, files }),

  unstageFiles: (projectPath: string, files: string[]): Promise<UnstageFilesResponse> =>
    invoke<UnstageFilesResponse>('unstage_files', { projectPath, files }),

  createCommit: (
    projectPath: string,
    message: string,
    amendLast?: boolean
  ): Promise<CreateCommitResponse> =>
    invoke<CreateCommitResponse>('create_commit', { projectPath, message, amendLast }),

  getBranches: (projectPath: string, includeRemote?: boolean): Promise<GetBranchesResponse> =>
    invoke<GetBranchesResponse>('get_branches', { projectPath, includeRemote }),

  createBranch: (
    projectPath: string,
    branchName: string,
    checkout?: boolean
  ): Promise<CreateBranchResponse> =>
    invoke<CreateBranchResponse>('create_branch', { projectPath, branchName, checkout }),

  switchBranch: (
    projectPath: string,
    branchName: string,
    force?: boolean
  ): Promise<SwitchBranchResponse> =>
    invoke<SwitchBranchResponse>('switch_branch', { projectPath, branchName, force }),

  deleteBranch: (
    projectPath: string,
    branchName: string,
    force?: boolean
  ): Promise<DeleteBranchResponse> =>
    invoke<DeleteBranchResponse>('delete_branch', { projectPath, branchName, force }),

  getCommitHistory: (
    projectPath: string,
    skip?: number,
    limit?: number,
    branch?: string
  ): Promise<GetCommitHistoryResponse> =>
    invoke<GetCommitHistoryResponse>('get_commit_history', {
      projectPath,
      skip,
      limit,
      branch,
    }),

  push: (
    projectPath: string,
    options?: {
      remote?: string;
      branch?: string;
      setUpstream?: boolean;
      force?: boolean;
    }
  ): Promise<GitPushResponse> =>
    invoke<GitPushResponse>('git_push', {
      projectPath,
      remote: options?.remote,
      branch: options?.branch,
      setUpstream: options?.setUpstream,
      force: options?.force,
    }),

  pull: (
    projectPath: string,
    options?: {
      remote?: string;
      branch?: string;
      rebase?: boolean;
    }
  ): Promise<GitPullResponse> =>
    invoke<GitPullResponse>('git_pull', {
      projectPath,
      remote: options?.remote,
      branch: options?.branch,
      rebase: options?.rebase,
    }),

  listStashes: (projectPath: string): Promise<ListStashesResponse> =>
    invoke<ListStashesResponse>('list_stashes', { projectPath }),

  createStash: (
    projectPath: string,
    message?: string,
    includeUntracked?: boolean
  ): Promise<CreateStashResponse> =>
    invoke<CreateStashResponse>('create_stash', { projectPath, message, includeUntracked }),

  applyStash: (projectPath: string, index?: number, pop?: boolean): Promise<ApplyStashResponse> =>
    invoke<ApplyStashResponse>('apply_stash', { projectPath, index, pop }),

  dropStash: (projectPath: string, index?: number): Promise<DropStashResponse> =>
    invoke<DropStashResponse>('drop_stash', { projectPath, index }),

  getRemotes: (projectPath: string): Promise<GetRemotesResponse> =>
    invoke<GetRemotesResponse>('get_remotes', { projectPath }),

  addRemote: (projectPath: string, name: string, url: string): Promise<AddRemoteResponse> =>
    invoke<AddRemoteResponse>('add_remote', { projectPath, name, url }),

  removeRemote: (projectPath: string, name: string): Promise<RemoveRemoteResponse> =>
    invoke<RemoveRemoteResponse>('remove_remote', { projectPath, name }),

  discardChanges: (projectPath: string, files: string[]): Promise<DiscardChangesResponse> =>
    invoke<DiscardChangesResponse>('discard_changes', { projectPath, files }),

  cleanUntracked: (
    projectPath: string,
    files: string[],
    includeDirectories?: boolean
  ): Promise<DiscardChangesResponse> =>
    invoke<DiscardChangesResponse>('clean_untracked', {
      projectPath,
      files,
      includeDirectories,
    }),

  fetch: (
    projectPath: string,
    options?: {
      remote?: string;
      allRemotes?: boolean;
      prune?: boolean;
    }
  ): Promise<GitFetchResponse> =>
    invoke<GitFetchResponse>('git_fetch', {
      projectPath,
      remote: options?.remote,
      allRemotes: options?.allRemotes,
      prune: options?.prune,
    }),

  rebase: (projectPath: string, onto: string): Promise<GitRebaseResponse> =>
    invoke<GitRebaseResponse>('git_rebase', { projectPath, onto }),

  rebaseAbort: (projectPath: string): Promise<GitRebaseResponse> =>
    invoke<GitRebaseResponse>('git_rebase_abort', { projectPath }),

  rebaseContinue: (projectPath: string): Promise<GitRebaseResponse> =>
    invoke<GitRebaseResponse>('git_rebase_continue', { projectPath }),

  getAuthStatus: (projectPath: string): Promise<GetGitAuthStatusResponse> =>
    invoke<GetGitAuthStatusResponse>('get_git_auth_status', { projectPath }),

  testRemoteConnection: (
    projectPath: string,
    remoteName: string
  ): Promise<TestRemoteConnectionResponse> =>
    invoke<TestRemoteConnectionResponse>('test_remote_connection', { projectPath, remoteName }),

  getFileDiff: (
    projectPath: string,
    filePath: string,
    staged: boolean
  ): Promise<GetFileDiffResponse> =>
    invoke<GetFileDiffResponse>('get_file_diff', { projectPath, filePath, staged }),
};

// ============================================================================
// Custom Step Template Commands
// ============================================================================

export type StepTemplateCategory =
  | 'package-manager'
  | 'git'
  | 'docker'
  | 'shell'
  | 'testing'
  | 'code-quality'
  | 'kubernetes'
  | 'database'
  | 'cloud'
  | 'ai'
  | 'security'
  | 'nodejs'
  | 'custom';

export interface CustomStepTemplate {
  id: string;
  name: string;
  command: string;
  category: StepTemplateCategory;
  description?: string;
  isCustom: boolean;
  createdAt: string;
}

export interface ListCustomTemplatesResponse {
  success: boolean;
  templates?: CustomStepTemplate[];
  error?: string;
}

export interface CustomTemplateResponse {
  success: boolean;
  template?: CustomStepTemplate;
  error?: string;
}

export const stepTemplateAPI = {
  loadCustomTemplates: (): Promise<ListCustomTemplatesResponse> =>
    invoke<ListCustomTemplatesResponse>('load_custom_step_templates'),

  saveCustomTemplate: (template: CustomStepTemplate): Promise<CustomTemplateResponse> =>
    invoke<CustomTemplateResponse>('save_custom_step_template', { template }),

  deleteCustomTemplate: (templateId: string): Promise<CustomTemplateResponse> =>
    invoke<CustomTemplateResponse>('delete_custom_step_template', { templateId }),
};

// ============================================================================
// Incoming Webhook API
// ============================================================================

import type { IncomingWebhookConfig, IncomingWebhookServerStatus } from '../types/incoming-webhook';

export type PortStatus = 'Available' | { InUseByWorkflow: string } | 'InUseByOther';

export const incomingWebhookAPI = {
  generateToken: (): Promise<string> => invoke<string>('generate_incoming_webhook_token'),

  generateSecret: (): Promise<string> => invoke<string>('generate_webhook_secret'),

  getServerStatus: (): Promise<IncomingWebhookServerStatus> =>
    invoke<IncomingWebhookServerStatus>('get_incoming_webhook_status'),

  createConfig: (): Promise<IncomingWebhookConfig> =>
    invoke<IncomingWebhookConfig>('create_incoming_webhook_config'),

  regenerateToken: (config: IncomingWebhookConfig): Promise<IncomingWebhookConfig> =>
    invoke<IncomingWebhookConfig>('regenerate_incoming_webhook_token', { config }),

  checkPortAvailable: (port: number, workflowId?: string): Promise<PortStatus> =>
    invoke<PortStatus>('check_port_available', { port, workflowId }),
};

// ============================================================================
// Keyboard Shortcuts API
// ============================================================================

import type { KeyboardShortcutsSettings } from '../types/shortcuts';

export const shortcutsAPI = {
  loadSettings: (): Promise<KeyboardShortcutsSettings> =>
    invoke<KeyboardShortcutsSettings>('load_keyboard_shortcuts'),

  saveSettings: (settings: KeyboardShortcutsSettings): Promise<void> =>
    invoke('save_keyboard_shortcuts', { settings }),

  registerGlobalToggle: (shortcutKey: string): Promise<void> =>
    invoke('register_global_toggle_shortcut', { shortcutKey }),

  unregisterGlobal: (): Promise<void> => invoke('unregister_global_shortcuts'),

  toggleWindowVisibility: (): Promise<boolean> => invoke<boolean>('toggle_window_visibility'),

  getRegisteredShortcuts: (): Promise<string[]> => invoke<string[]>('get_registered_shortcuts'),

  isShortcutRegistered: (shortcutKey: string): Promise<boolean> =>
    invoke<boolean>('is_shortcut_registered', { shortcutKey }),
};

export const shortcutsEvents = {
  onGlobalShortcutTriggered: (callback: (action: string) => void): Promise<UnlistenFn> =>
    listen<string>('global-shortcut-triggered', (event) => callback(event.payload)),
};

// ============================================================================
// MCP Server Integration
// ============================================================================

export interface McpServerInfo {
  binary_path: string;
  name: string;
  version: string;
  is_available: boolean;
  config_json: string;
  config_toml: string;
  env_type: string;
}

export interface McpToolInfo {
  name: string;
  description: string;
  category: string;
  permissionCategory: 'read' | 'execute' | 'write';
  applicablePermissions: ('read' | 'execute' | 'write')[];
}

export type McpPermissionMode = 'read_only' | 'execute_with_confirm' | 'full_access';

export type DevServerMode = 'mcp_managed' | 'ui_integrated' | 'reject_with_hint';

export interface McpServerConfig {
  isEnabled: boolean;
  permissionMode: McpPermissionMode;
  devServerMode: DevServerMode;
  allowedTools: string[];
  logRequests: boolean;
}

export type McpToolCategory = 'read' | 'write' | 'execute';

export interface McpToolWithPermission {
  name: string;
  description: string;
  category: McpToolCategory;
  isAllowed: boolean;
}

export interface McpLogEntry {
  timestamp: string;
  tool: string;
  arguments: Record<string, unknown>;
  result: string;
  durationMs: number;
  error: string | null;
}

export interface McpLogsResponse {
  entries: McpLogEntry[];
  totalCount: number;
}

export interface McpHealthCheckResult {
  isHealthy: boolean;
  version: string | null;
  responseTimeMs: number;
  error: string | null;
  binaryPath: string;
  envType: string;
}

export const mcpAPI = {
  getServerInfo: (): Promise<McpServerInfo> => invoke<McpServerInfo>('get_mcp_server_info'),

  testConnection: (): Promise<McpHealthCheckResult> =>
    invoke<McpHealthCheckResult>('test_mcp_connection'),

  getTools: (): Promise<McpToolInfo[]> => invoke<McpToolInfo[]>('get_mcp_tools'),

  getConfig: (): Promise<McpServerConfig> => invoke<McpServerConfig>('get_mcp_config'),

  saveConfig: (config: McpServerConfig): Promise<void> => invoke('save_mcp_config', { config }),

  updateConfig: (options: {
    isEnabled?: boolean;
    permissionMode?: McpPermissionMode;
    devServerMode?: DevServerMode;
    allowedTools?: string[];
    logRequests?: boolean;
  }): Promise<McpServerConfig> => invoke<McpServerConfig>('update_mcp_config', options),

  getToolsWithPermissions: (): Promise<McpToolWithPermission[]> =>
    invoke<McpToolWithPermission[]>('get_mcp_tools_with_permissions'),

  getLogs: (limit?: number): Promise<McpLogsResponse> =>
    invoke<McpLogsResponse>('get_mcp_logs', { limit }),

  clearLogs: (): Promise<void> => invoke('clear_mcp_logs'),
};

// ============================================================================
// MCP Action API (021-mcp-actions)
// ============================================================================

import type {
  MCPAction,
  MCPActionPermission,
  MCPActionExecution,
  MCPActionType,
  PermissionLevel,
  ExecutionStatus as MCPExecutionStatus,
} from '../types/mcp-action';

export type { MCPAction, MCPActionPermission, MCPActionExecution, MCPActionType, PermissionLevel };
export type { MCPExecutionStatus };

export interface PendingActionRequest {
  executionId: string;
  actionId: string | null;
  actionType: string;
  actionName: string;
  description: string;
  parameters: Record<string, unknown> | null;
  sourceClient: string | null;
  startedAt: string;
}

export interface ActionRequestResponse {
  executionId: string;
  approved: boolean;
  status: string;
}

export const mcpActionAPI = {
  // Actions CRUD
  listActions: (
    projectId?: string,
    actionType?: MCPActionType,
    isEnabled?: boolean
  ): Promise<MCPAction[]> =>
    invoke<MCPAction[]>('list_mcp_actions', { projectId, actionType, isEnabled }),

  getAction: (actionId: string): Promise<MCPAction | null> =>
    invoke<MCPAction | null>('get_mcp_action', { actionId }),

  createAction: (
    actionType: MCPActionType,
    name: string,
    description: string | null,
    config: Record<string, unknown>,
    projectId?: string
  ): Promise<MCPAction> =>
    invoke<MCPAction>('create_mcp_action', { actionType, name, description, config, projectId }),

  updateAction: (
    actionId: string,
    name?: string,
    description?: string,
    config?: Record<string, unknown>,
    isEnabled?: boolean
  ): Promise<MCPAction> =>
    invoke<MCPAction>('update_mcp_action', { actionId, name, description, config, isEnabled }),

  deleteAction: (actionId: string): Promise<boolean> =>
    invoke<boolean>('delete_mcp_action', { actionId }),

  // Executions
  getExecutions: (
    actionId?: string,
    actionType?: MCPActionType,
    status?: MCPExecutionStatus,
    limit?: number
  ): Promise<MCPActionExecution[]> =>
    invoke<MCPActionExecution[]>('get_mcp_action_executions', {
      actionId,
      actionType,
      status,
      limit,
    }),

  getExecution: (executionId: string): Promise<MCPActionExecution | null> =>
    invoke<MCPActionExecution | null>('get_mcp_action_execution', { executionId }),

  cleanupExecutions: (keepCount?: number, maxAgeDays?: number): Promise<number> =>
    invoke<number>('cleanup_mcp_action_executions', { keepCount, maxAgeDays }),

  // Permissions
  listPermissions: (): Promise<MCPActionPermission[]> =>
    invoke<MCPActionPermission[]>('list_mcp_action_permissions'),

  updatePermission: (
    actionId: string | null,
    actionType: MCPActionType | null,
    permissionLevel: PermissionLevel
  ): Promise<MCPActionPermission> =>
    invoke<MCPActionPermission>('update_mcp_action_permission', {
      actionId,
      actionType,
      permissionLevel,
    }),

  deletePermission: (permissionId: string): Promise<boolean> =>
    invoke<boolean>('delete_mcp_action_permission', { permissionId }),

  // Pending requests (user confirmation)
  getPendingRequests: (): Promise<PendingActionRequest[]> =>
    invoke<PendingActionRequest[]>('get_pending_action_requests'),

  respondToRequest: (
    executionId: string,
    approved: boolean,
    reason?: string
  ): Promise<ActionRequestResponse> =>
    invoke<ActionRequestResponse>('respond_to_action_request', { executionId, approved, reason }),
};

// ============================================================================
// Unified API Export
// ============================================================================

export const tauriAPI = {
  ...scriptAPI,
  ...workflowAPI,
  ...worktreeAPI,
  ...ipaAPI,
  ...settingsAPI,
  ...notificationAPI,
  ...notificationHistoryAPI,
  ...gitAPI,
  ...stepTemplateAPI,
  ...shortcutsAPI,
  ...mcpAPI,
  ...mcpActionAPI,
};
