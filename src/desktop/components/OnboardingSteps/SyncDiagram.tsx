import { targetLabel } from './onboarding-cli';

export interface DiagramTarget {
  name: string;
  /** Flips to a check once this target's sync result has arrived. */
  done: boolean;
  /** Skills linked, when the CLI reported it. */
  linked?: number;
}

const MAX_NODES = 6;
const W = 560;
const NODE_W = 150;
const NODE_H = 40;
const ROW = 50;
const SRC_X = 20;
const DST_X = W - 20 - NODE_W;

interface Node {
  key: string;
  label: string;
  sub: string;
  done: boolean;
}

function toNodes(targets: DiagramTarget[]): Node[] {
  const sub = (t: { done: boolean; linked?: number }) =>
    !t.done ? 'Syncing…' : t.linked === undefined ? 'Synced' : `${t.linked} skills linked`;
  const shown = targets.length > MAX_NODES ? targets.slice(0, MAX_NODES - 1) : targets;
  const nodes = shown.map((t) => ({
    key: t.name,
    label: targetLabel(t.name),
    sub: sub(t),
    done: t.done,
  }));
  if (shown.length < targets.length) {
    const rest = targets.slice(shown.length);
    const done = rest.every((t) => t.done);
    const linked = rest.some((t) => t.linked === undefined)
      ? undefined
      : rest.reduce((n, t) => n + (t.linked ?? 0), 0);
    nodes.push({ key: '_rest', label: `+${rest.length} more`, sub: sub({ done, linked }), done });
  }
  return nodes;
}

/**
 * Source node on the left, one node per target on the right. Wires draw in sequence, then a
 * stream of dots flows along each wire until that target's result arrives and it flips to a check.
 */
export default function SyncDiagram({ targets }: { targets: DiagramTarget[] }) {
  const nodes = toNodes(targets);
  const H = Math.max(nodes.length * ROW, 70) + 10;
  const sy = H / 2;
  const summary = `Syncing your skills to ${targets.length} ${targets.length === 1 ? 'target' : 'targets'}`;

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      role="img"
      aria-label={summary}
      className="h-full max-h-full w-full"
      preserveAspectRatio="xMidYMid meet"
    >
      {nodes.map((n, i) => {
        const ty = 5 + i * ROW + NODE_H / 2 + (H - 10 - nodes.length * ROW) / 2;
        const d = `M${SRC_X + 110} ${sy} C ${W / 2} ${sy}, ${W / 2} ${ty}, ${DST_X} ${ty}`;
        const style = { ['--ob-i' as string]: i };
        return (
          <g key={n.key}>
            <path className="ob-wire" d={d} pathLength={1} style={style} />
            <path
              className="ob-flow"
              d={d}
              pathLength={1}
              style={style}
              data-done={String(n.done)}
            />
            <g className="ob-node-in" style={{ ['--ob-i' as string]: i + 2 }}>
              <rect
                className="ob-tnode"
                data-done={String(n.done)}
                x={DST_X}
                y={ty - NODE_H / 2}
                width={NODE_W}
                height={NODE_H}
                rx={10}
              />
              <text x={DST_X + 14} y={ty - 3} fontSize={12.5} fontWeight={700} fill="var(--ink)">
                {n.label}
              </text>
              <text x={DST_X + 14} y={ty + 12} fontSize={10.5} fill="var(--ink-2)">
                {n.sub}
              </text>
              <g className="ob-tcheck" data-done={String(n.done)}>
                <circle cx={DST_X + NODE_W - 20} cy={ty} r={10} fill="var(--ok-bg)" />
                <path d={`M${DST_X + NODE_W - 25} ${ty} l3.5 3.5 l7 -7`} />
              </g>
            </g>
          </g>
        );
      })}
      <g className="ob-node-in" style={{ ['--ob-i' as string]: 0 }}>
        <rect x={SRC_X} y={sy - 28} width={110} height={56} rx={12} fill="var(--pri)" />
        <text
          x={SRC_X + 55}
          y={sy - 2}
          textAnchor="middle"
          fontSize={13}
          fontWeight={700}
          fill="var(--on-pri)"
        >
          Your skills
        </text>
        <text
          x={SRC_X + 55}
          y={sy + 14}
          textAnchor="middle"
          fontSize={10.5}
          fill="var(--on-pri)"
          opacity={0.75}
        >
          source
        </text>
      </g>
    </svg>
  );
}
