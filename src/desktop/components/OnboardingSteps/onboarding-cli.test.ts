import { describe, expect, it } from 'vitest';
import { parseDetectedTargets, parseSyncResult, parseTargetNames } from './onboarding-cli';

describe('parseDetectedTargets', () => {
  it('returns the agents the dry run would add and skips undetected ones', () => {
    const output = `Discovering new agents
✓ Found 4 new agent(s)
! Agent not detected: codex (skipped)
! Dry run - would add 2 agent(s) to config
  + claude
  + cursor`;
    expect(parseDetectedTargets(output).map((t) => t.name)).toEqual(['claude', 'cursor']);
  });

  it('returns nothing when no agent is selected', () => {
    expect(parseDetectedTargets('✓ Found 4 new agent(s)\n→ No agents selected')).toEqual([]);
  });
});

describe('parseSyncResult', () => {
  it('reads skills linked per target from sync --json', () => {
    const json = JSON.stringify({ details: [{ name: 'claude', linked: 2 }, { name: 'cursor' }] });
    expect(parseSyncResult(json)?.perTarget).toEqual({ claude: 2, cursor: 0 });
  });

  it('returns null for non-JSON output', () => {
    expect(parseSyncResult('Synced 2 targets')).toBeNull();
  });
});

describe('parseTargetNames', () => {
  it('lists target names from target list --json', () => {
    expect(parseTargetNames('{"targets":[{"name":"claude"},{"name":"codex"}]}')).toEqual([
      'claude',
      'codex',
    ]);
  });
});
