import type { AuditFinding, AvailableUpdates, SourceHealth } from '../../api/tauri-bridge';

export type StatusTone = 'bad' | 'accent' | 'warn' | 'neutral' | 'ok';
export type StatusIcon = 'shield' | 'update' | 'sync' | 'upload' | 'server';

/** `open` goes to the Web UI page where the user acts; only the app's own server restarts here. */
export type RowAction = { kind: 'open'; path: string; label: string } | { kind: 'restart' };

export interface StatusRow {
  id: string;
  tone: StatusTone;
  icon: StatusIcon;
  title: string;
  detail: string;
  action?: RowAction;
}

export interface ServerState {
  running: boolean;
  port: number | null;
  stopped: boolean;
}

function plural(n: number, one: string, many = `${one}s`) {
  return `${n} ${n === 1 ? one : many}`;
}

function isAlert(finding: AuditFinding) {
  return finding.severity === 'CRITICAL' || finding.severity === 'HIGH';
}

/** The Web UI page to review findings: the one skill or agent they are in, else the audit page. */
function reviewPath(findings: AuditFinding[]) {
  const resources = new Set(findings.map((f) => `${f.kind}:${f.skill}`));
  if (resources.size !== 1) return '/audit';
  const [finding] = findings;
  return `/${finding.kind === 'agent' ? 'agents' : 'skills'}/${encodeURIComponent(finding.skill)}`;
}

function auditRow(findings: AuditFinding[]): StatusRow | null {
  if (findings.length === 0) return null;
  const alerts = findings.filter(isAlert);
  const shown = alerts.length > 0 ? alerts : findings;
  return {
    id: 'audit',
    tone: alerts.length > 0 ? 'bad' : 'neutral',
    icon: 'shield',
    title:
      alerts.length > 0
        ? plural(alerts.length, 'security issue')
        : plural(findings.length, 'audit finding'),
    detail: [...new Set(shown.map((f) => f.skill))].join(', '),
    action: { kind: 'open', path: reviewPath(shown), label: 'Review' },
  };
}

/** New app and CLI versions, each opening the settings tab that installs it. */
function versionRows(updates: AvailableUpdates): StatusRow[] {
  const versions = [
    { id: 'app-update', name: 'Skillshare App', version: updates.app, tab: 'about' },
    { id: 'cli-update', name: 'skillshare CLI', version: updates.cli, tab: 'cli' },
  ];
  return versions
    .filter(({ version }) => version)
    .map(
      ({ id, name, version, tab }): StatusRow => ({
        id,
        tone: 'accent',
        icon: 'update',
        title: `${name} v${version?.replace(/^v/, '')} available`,
        detail: 'A new version is ready to install',
        action: { kind: 'open', path: `/settings?tab=${tab}`, label: 'Update' },
      })
    );
}

/** One row per kind with updates, each opening the Web UI page that updates that kind. */
function updateRows(updates: AvailableUpdates): StatusRow[] {
  const kinds = [
    { id: 'skill', names: updates.skills, one: 'skill update', path: '/skills?tab=updates' },
    { id: 'plugin', names: updates.plugins, one: 'plugin update', path: '/plugins' },
    { id: 'agent', names: updates.agents, one: 'agent update', path: '/agents?tab=updates' },
    {
      id: 'repository',
      names: updates.repositories,
      one: 'repository update',
      many: 'repository updates',
      path: '/skills?tab=updates',
    },
  ];
  return kinds
    .filter((kind) => kind.names.length > 0)
    .map(
      (kind): StatusRow => ({
        id: `${kind.id}-updates`,
        tone: 'accent',
        icon: 'update',
        title: plural(kind.names.length, kind.one, kind.many),
        detail: kind.names.join(', '),
        action: { kind: 'open', path: kind.path, label: 'Update' },
      })
    );
}

function sourceRows(health: SourceHealth): StatusRow[] {
  const rows: StatusRow[] = [];
  const targets = health.outOfSyncTargets;
  if (targets.length > 0) {
    rows.push({
      id: 'sync',
      tone: 'warn',
      icon: 'sync',
      title: `${plural(targets.length, 'target')} ${targets.length === 1 ? 'needs' : 'need'} a sync`,
      detail: targets.join(', '),
      action: { kind: 'open', path: '/sync', label: 'Sync' },
    });
  }
  const git = health.git;
  if (git && git.uncommitted + git.ahead > 0) {
    rows.push({
      id: 'push',
      tone: 'neutral',
      icon: 'upload',
      title: `${plural(git.uncommitted + git.ahead, 'change')} not pushed`,
      detail: [
        git.uncommitted > 0 && plural(git.uncommitted, 'uncommitted change'),
        git.ahead > 0 && plural(git.ahead, 'commit'),
      ]
        .filter(Boolean)
        .join(', '),
      action: { kind: 'open', path: '/git', label: 'Push' },
    });
  }
  if (git && git.behind > 0) {
    rows.push({
      id: 'pull',
      tone: 'neutral',
      icon: 'upload',
      title: `${plural(git.behind, 'update')} to pull`,
      detail: `${plural(git.behind, 'commit')} on the remote`,
      action: { kind: 'open', path: '/git', label: 'Pull' },
    });
  }
  return rows;
}

function serverRow(server: ServerState): StatusRow | null {
  if (server.stopped) {
    return {
      id: 'server',
      tone: 'bad',
      icon: 'server',
      title: 'Server stopped',
      detail: 'It kept exiting',
      action: { kind: 'restart' },
    };
  }
  if (!server.running) return null;
  return {
    id: 'server',
    tone: 'ok',
    icon: 'server',
    title: 'Server running',
    detail: `Port ${server.port}`,
  };
}

/** The status panel rows, most severe first; the server is always last. */
export function statusRows(
  findings: AuditFinding[],
  updates: AvailableUpdates,
  health: SourceHealth,
  server: ServerState
): StatusRow[] {
  return [
    auditRow(findings),
    ...versionRows(updates),
    ...updateRows(updates),
    ...sourceRows(health),
    serverRow(server),
  ].filter((row): row is StatusRow => row !== null);
}

/** The title bar label: security issues lead, then everything else with an action. */
export function statusSummary(findings: AuditFinding[], rows: StatusRow[]) {
  const actionable = rows.filter((row) => row.action).length;
  if (actionable === 0) return null;
  const alerts = findings.filter(isAlert).length;
  if (alerts > 0) {
    return { security: true, label: plural(alerts, 'security issue'), more: actionable - 1 };
  }
  return { security: false, label: `${plural(actionable, 'thing')} to review`, more: 0 };
}
