import { invoke } from '@tauri-apps/api/core';

export interface Project {
  id: string;
  name: string;
  path: string;
  projectType: 'global' | 'project';
  addedAt: string;
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
  checkCliUpdate: () => invoke<string | null>('check_cli_update'),
  upgradeCli: () => invoke<string>('upgrade_cli'),
  runCli: (cliPath: string, args: string[], workingDir?: string) =>
    invoke<string>('run_cli', { cliPath, args, workingDir }),

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
  restartServer: (cliPath: string, projectDir?: string) =>
    invoke<number>('restart_server', { cliPath, projectDir }),
  healthCheck: () => invoke<boolean>('server_health_check'),
  getServerPort: () => invoke<number>('get_server_port'),

  // App commands
  getAppState: () => invoke<AppInfo>('get_app_state'),
  getOnboardingStatus: () => invoke<OnboardingStatus>('get_onboarding_status'),
  getPreferredPort: () => invoke<number>('get_preferred_port'),
  setPreferredPort: (port: number) => invoke<void>('set_preferred_port', { port }),
  getPreferredTheme: () => invoke<string>('get_preferred_theme'),
  setPreferredTheme: (theme: string) => invoke<void>('set_preferred_theme', { theme }),
  getNotifySync: () => invoke<boolean>('get_notify_sync'),
  setNotifySync: (enabled: boolean) => invoke<void>('set_notify_sync', { enabled }),
  getNotifyUpdate: () => invoke<boolean>('get_notify_update'),
  setNotifyUpdate: (enabled: boolean) => invoke<void>('set_notify_update', { enabled }),
  resetAllData: () => invoke<void>('reset_all_data'),
};
