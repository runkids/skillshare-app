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
  /** Skills and tracked repos in the active project with upstream changes. */
  skills: string[];
}

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

/** Emitted after a tray or auto Quick Sync succeeds. */
export const SYNC_COMPLETED_EVENT = 'sync-completed';

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

  // Activity commands
  getActivity: () => invoke<ActivityEntry[]>('get_activity'),

  // Terminal commands
  getPtyEnv: () => invoke<Record<string, string>>('get_pty_env'),

  // Utility commands
  getGlobalConfigDir: (cliPath: string) => invoke<string>('get_global_config_dir', { cliPath }),
};
