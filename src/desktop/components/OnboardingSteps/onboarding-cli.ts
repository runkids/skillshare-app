export interface KnownTarget {
  name: string;
  label: string;
  path: string;
}

// Names and global skills paths from skillshare's targets.yaml. `init --discover` does not list
// detected agents by name, so these are probed with `--select ... --dry-run`.
export const KNOWN_TARGETS: KnownTarget[] = [
  { name: 'claude', label: 'Claude Code', path: '~/.claude/skills' },
  { name: 'codex', label: 'Codex', path: '~/.agents/skills' },
  { name: 'cursor', label: 'Cursor', path: '~/.cursor/skills' },
  { name: 'gemini', label: 'Gemini CLI', path: '~/.gemini/skills' },
  { name: 'opencode', label: 'OpenCode', path: '~/.config/opencode/skills' },
  { name: 'copilot', label: 'GitHub Copilot', path: '~/.copilot/skills' },
  { name: 'windsurf', label: 'Windsurf', path: '~/.codeium/windsurf/skills' },
  { name: 'cline', label: 'Cline', path: '~/.cline/skills' },
  { name: 'continue', label: 'Continue', path: '~/.continue/skills' },
  { name: 'roo', label: 'Roo Code', path: '~/.roo/skills' },
  { name: 'kiro', label: 'Kiro', path: '~/.kiro/skills' },
  { name: 'trae', label: 'Trae', path: '~/.trae/skills' },
  { name: 'junie', label: 'Junie', path: '~/.junie/skills' },
  { name: 'amp', label: 'Amp', path: '~/.config/agents/skills' },
  { name: 'augment', label: 'Augment', path: '~/.augment/skills' },
  { name: 'droid', label: 'Droid', path: '~/.factory/skills' },
  { name: 'qwen', label: 'Qwen Code', path: '~/.qwen/skills' },
];

/** Args that make the CLI report which of the known agents are installed, without changing anything. */
export const DETECT_TARGETS_ARGS = [
  'init',
  '--discover',
  '--select',
  KNOWN_TARGETS.map((t) => t.name).join(','),
  '--dry-run',
];

/** Parse `+ name` lines from `init --discover --select ... --dry-run`. */
export function parseDetectedTargets(output: string): KnownTarget[] {
  const names = new Set(
    output
      .split('\n')
      .map((line) => /^\s*\+\s+(\S+)\s*$/.exec(line)?.[1])
      .filter((n): n is string => Boolean(n))
  );
  return KNOWN_TARGETS.filter((t) => names.has(t.name));
}

/** Names from `target list --json`. */
export function parseTargetNames(output: string): string[] {
  const parsed = JSON.parse(output) as { targets?: { name: string }[] };
  return (parsed.targets ?? []).map((t) => t.name);
}

export interface SyncSummary {
  /** Skills linked per target name, from `sync --json` details. */
  perTarget: Record<string, number>;
}

/** Parse `sync --json`; returns null when the output is not the JSON report. */
export function parseSyncResult(output: string): SyncSummary | null {
  try {
    const parsed = JSON.parse(output) as {
      details?: { name: string; linked?: number }[];
    };
    if (!Array.isArray(parsed.details)) return null;
    return {
      perTarget: Object.fromEntries(parsed.details.map((d) => [d.name, d.linked ?? 0])),
    };
  } catch {
    return null;
  }
}

export function targetLabel(name: string): string {
  return KNOWN_TARGETS.find((t) => t.name === name)?.label ?? name;
}
