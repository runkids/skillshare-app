import { invoke } from '@tauri-apps/api/core';

export interface Project {
  id: string;
  name: string;
  path: string;
  projectType: 'global' | 'project';
  addedAt: string;
}

export interface InstallPlatform {
  os: 'macos' | 'linux' | 'windows';
  arch: 'arm64' | 'x64';
  brew: boolean;
}

export interface AvailableUpdates {
  cli: string | null;
  app: string | null;
  /** Per-kind resources in the active project with upstream changes. */
  skills: string[];
  repositories: string[];
  agents: string[];
  plugins: string[];
}

export interface SourceGitState {
  uncommitted: number;
  ahead: number;
  behind: number;
}

export interface SourceHealth {
  /** Skills that exist only in a target, which collect would copy into the source. */
  localSkills: string[];
  /** Targets that sync would change. */
  outOfSyncTargets: string[];
  /** Git state of the source; null outside a git repo and in project mode. */
  git: SourceGitState | null;
}

export const SOURCE_HEALTH_EVENT = 'source-health';

/** A finding from the security audit run after Update All, Pull or auto-sync. */
export interface AuditFinding {
  skill: string;
  kind: 'skill' | 'agent';
  severity: 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW';
  message: string;
  file: string;
  /** 0 when the finding is about the file or skill as a whole. */
  line: number;
}

export const AUDIT_EVENT = 'audit-report';

/** An action the title bar status panel runs through the same code as the tray. */
export type StatusAction = 'update_all' | 'sync' | 'collect' | 'push' | 'pull';

export interface PathHint {
  dir: string;
  command: string;
}

export interface TerminalAccess {
  needsLink: boolean;
  linked: boolean;
  pathHint: PathHint | null;
}

export interface InstallResult {
  path: string;
  version: string;
}

export interface InstallOutput {
  stream: 'stdout' | 'stderr';
  line: string;
}

export const CLI_INSTALL_OUTPUT_EVENT = 'cli-install-output';
export const QUICK_ACTIONS_OPENED_EVENT = 'quick-actions-opened';

export interface QuickActionsSettings {
  enabled: boolean;
  shortcut: string;
  error: string | null;
}

export interface QuickActionsContext {
  projectId: string;
  projectName: string;
  sourceDir: string;
}

export interface SkillSearchResult {
  name: string;
  description: string;
  source: string;
  skill: string;
}

/** Emitted after a tray or auto Quick Sync succeeds. */
export const SYNC_COMPLETED_EVENT = 'sync-completed';
export const TRAY_PROJECT_REQUESTED_EVENT = 'tray-project-requested';
export const AUTO_SYNC_CHANGED_EVENT = 'auto-sync-changed';

/** One operation from `skillshare log`, newest first. */
export interface ActivityEntry {
  /** RFC 3339 with the CLI's offset. */
  ts: string;
  cmd: string;
  status: 'ok' | 'error' | 'partial' | 'blocked';
  message: string | null;
  durationMs: number | null;
  /** Skills or targets the operation touched. */
  subjects: string[];
  /** A count summary such as "7 targets, 1 failed". */
  detail: string | null;
}

export interface OnboardingStatus {
  completed: boolean;
  cliReady: boolean;
  firstProjectCreated: boolean;
  firstSyncDone: boolean;
}

export interface AppInfo {
  cliVersion: string | null;
  cliSource: string | null;
  serverRunning: boolean;
  serverPort: number | null;
  onboarding: OnboardingStatus;
}

export const tauriBridge = {
  // Quick Actions commands
  getQuickActionsSettings: () => invoke<QuickActionsSettings>('get_quick_actions_settings'),
  setQuickActionsSettings: (enabled: boolean, shortcut: string) =>
    invoke<void>('set_quick_actions_settings', { enabled, shortcut }),
  getQuickActionsContext: () => invoke<QuickActionsContext>('get_quick_actions_context'),
  closeQuickActions: () => invoke<void>('close_quick_actions'),
  quickSearch: (query: string, projectId: string) =>
    invoke<SkillSearchResult[]>('quick_search', { query, projectId }),
  quickInstall: (source: string, skill: string, projectId: string) =>
    invoke<string>('quick_install', { source, skill, projectId }),
  quickNewSkill: (name: string, projectId: string) =>
    invoke<{ path: string; openError: string | null }>('quick_new_skill', { name, projectId }),

  // CLI commands
  detectCli: () => invoke<string | null>('detect_cli'),
  getCliVersion: (cliPath: string) => invoke<string>('get_cli_version', { cliPath }),
  downloadCli: () => invoke<string>('download_cli'),
  upgradeCli: () => invoke<string>('upgrade_cli'),
  runCli: (cliPath: string, args: string[], workingDir?: string) =>
    invoke<string>('run_cli', { cliPath, args, workingDir }),
  detectInstallPlatform: () => invoke<InstallPlatform>('detect_install_platform'),
  cliTerminalAccess: (cliPath: string) =>
    invoke<TerminalAccess>('cli_terminal_access', { cliPath }),
  linkCliForTerminal: (cliPath: string) =>
    invoke<TerminalAccess>('link_cli_for_terminal', { cliPath }),
  installCli: (method: string) => invoke<InstallResult>('install_cli', { method }),
  cancelCliInstall: () => invoke<boolean>('cancel_cli_install'),

  // Project commands
  listProjects: () => invoke<Project[]>('list_projects'),
  getActiveProject: () => invoke<Project | null>('get_active_project'),
  addProject: (name: string, path: string, projectType: 'global' | 'project') =>
    invoke<Project>('add_project', { name, path, projectType }),
  removeProject: (id: string) => invoke<void>('remove_project', { id }),
  switchProject: (id: string) => invoke<void>('switch_project', { id }),

  // Server commands
  startServer: (cliPath: string, projectDir?: string) =>
    invoke<number>('start_server', { cliPath, projectDir }),
  stopServer: () => invoke<void>('stop_server'),
  healthCheck: () => invoke<boolean>('server_health_check'),
  getServerPort: () => invoke<number>('get_server_port'),

  // App commands
  getAppState: () => invoke<AppInfo>('get_app_state'),
  getPreferredPort: () => invoke<number>('get_preferred_port'),
  setPreferredPort: (port: number) => invoke<void>('set_preferred_port', { port }),
  getNotifySync: () => invoke<boolean>('get_notify_sync'),
  setNotifySync: (enabled: boolean) => invoke<void>('set_notify_sync', { enabled }),
  getAutoSync: () => invoke<boolean>('get_auto_sync'),
  setAutoSync: (enabled: boolean) => invoke<void>('set_auto_sync', { enabled }),
  getAvailableUpdates: () => invoke<AvailableUpdates>('get_available_updates'),
  openLogsFolder: () => invoke<void>('open_logs_folder'),
  exportDiagnostics: () => invoke<string>('export_diagnostics'),
  checkUpdatesNow: () => invoke<AvailableUpdates>('check_updates_now'),
  getNotifyUpdate: () => invoke<boolean>('get_notify_update'),
  setNotifyUpdate: (enabled: boolean) => invoke<void>('set_notify_update', { enabled }),
  resetAllData: () => invoke<void>('reset_all_data'),
  getSourceHealth: () => invoke<SourceHealth>('get_source_health'),
  getAuditReport: () => invoke<AuditFinding[]>('get_audit_report'),
  runStatusAction: (action: StatusAction) => invoke<void>('run_status_action', { action }),
  checkStatusNow: () => invoke<void>('check_status_now'),

  // Activity commands
  getActivity: () => invoke<ActivityEntry[]>('get_activity'),

  // Terminal commands
  getPtyEnv: () => invoke<Record<string, string>>('get_pty_env'),

  // Utility commands
  getGlobalConfigDir: (cliPath: string) => invoke<string>('get_global_config_dir', { cliPath }),
};
