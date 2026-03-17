/**
 * PackageFlow Type Definitions
 */
export type {
  Workflow,
  WorkflowNode,
  ScriptNodeConfig,
  NodePosition,
  Execution,
  ExecutionStatus,
  NodeResult,
  NodeStatus,
  WorkflowStore,
  UserSettings,
  NodeStartedEvent,
  OutputEvent,
  NodeCompletedEvent,
  ExecutionCompletedEvent,
  ExecutionPausedEvent,
  LoadAllResponse,
  SaveResponse,
  DeleteResponse,
  ExecuteResponse,
  CancelResponse,
  ContinueResponse,
  RunningExecution,
  GetRunningResponse,
  RestoreRunningResponse,
  KillProcessResponse,
  WorkflowAPI,
} from './workflow';

export type {
  AppSettings,
  StoreData,
  ScriptOutputPayload,
  ScriptCompletedPayload,
  NodeStartedPayload,
  ExecutionOutputPayload,
  NodeCompletedPayload,
  ExecutionCompletedPayload,
  ExecutionPausedPayload,
  IpaScanProgressPayload,
} from './tauri';

export interface IpaResult {
  fileName: string;
  filePath: string;
  bundleId: string;
  version: string;
  build: string;
  displayName: string;
  deviceCapabilities: string;
  error: string | null;
  fullPlist: Record<string, unknown> | null;
  createdAt: string;
}

export interface ColumnConfig {
  key: keyof IpaResult | 'error';
  label: string;
  fullKey?: string;
  isReadOnly?: boolean;
  isStatus?: boolean;
}

export interface SigningIdentity {
  hash: string;
  name: string;
}

export interface ModalData {
  rowIndex: number;
  colIndex: number;
  column: ColumnConfig;
  result: IpaResult;
  value: string;
  isReadOnly: boolean;
  plistKey?: string;
}

export interface CheckHasIpaFilesResponse {
  success: boolean;
  hasIpaFiles: boolean;
  count: number;
}

export interface ScanProjectIpaResponse {
  success: boolean;
  results: IpaResult[];
  error?: string;
}

export interface KillAllNodeProcessesResponse {
  success: boolean;
  message?: string;
  error?: string;
}
