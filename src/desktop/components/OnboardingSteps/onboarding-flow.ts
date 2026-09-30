export const STEP_LABELS = ['Get the CLI', 'Initialize', 'First sync'] as const;

export interface FlowState {
  step: number;
  cliPath: string | null;
  complete: boolean;
}

export type FlowAction =
  | { type: 'cli-ready'; cliPath: string }
  | { type: 'init-done' }
  | { type: 'sync-done' };

export const initialFlow: FlowState = { step: 0, cliPath: null, complete: false };

export function flowReducer(state: FlowState, action: FlowAction): FlowState {
  switch (action.type) {
    case 'cli-ready':
      return state.step === 0 ? { ...state, step: 1, cliPath: action.cliPath } : state;
    case 'init-done':
      return state.step === 1 ? { ...state, step: 2 } : state;
    case 'sync-done':
      return state.step === 2 ? { ...state, complete: true } : state;
  }
}

export type StepStatus = 'done' | 'current' | 'pending';

export function stepStatus(state: FlowState, index: number): StepStatus {
  if (index < state.step || (state.complete && index === state.step)) return 'done';
  return index === state.step ? 'current' : 'pending';
}
