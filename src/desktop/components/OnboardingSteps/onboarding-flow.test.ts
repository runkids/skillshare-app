import { describe, expect, it } from 'vitest';
import { flowReducer, initialFlow, stepStatus } from './onboarding-flow';

describe('onboarding flow', () => {
  it('moves cli -> init -> sync -> complete', () => {
    let s = flowReducer(initialFlow, { type: 'cli-ready', cliPath: '/bin/skillshare' });
    expect(s).toMatchObject({ step: 1, cliPath: '/bin/skillshare', complete: false });
    s = flowReducer(s, { type: 'init-done' });
    expect(s.step).toBe(2);
    s = flowReducer(s, { type: 'sync-done' });
    expect(s).toMatchObject({ step: 2, complete: true });
  });

  it('ignores actions that arrive out of order', () => {
    expect(flowReducer(initialFlow, { type: 'init-done' })).toBe(initialFlow);
    expect(flowReducer(initialFlow, { type: 'sync-done' })).toBe(initialFlow);
  });

  it('marks earlier steps done, the current one current, later ones pending', () => {
    const s = { ...initialFlow, step: 1 };
    expect([0, 1, 2].map((i) => stepStatus(s, i))).toEqual(['done', 'current', 'pending']);
  });

  it('marks the last step done once the sync completes', () => {
    expect(stepStatus({ ...initialFlow, step: 2, complete: true }, 2)).toBe('done');
  });
});
