import { describe, expect, it } from 'vitest';
import type { ActivityEntry } from '../api/tauri-bridge';
import { dayLabel, groupByDay, relativeTime } from './activity';

const now = new Date(2026, 9, 1, 15, 0);

function entry(ts: Date): ActivityEntry {
  return {
    ts: ts.toISOString(),
    cmd: 'sync',
    status: 'ok',
    message: null,
    durationMs: null,
    subjects: [],
    detail: null,
  };
}

describe('dayLabel', () => {
  it('names today and yesterday', () => {
    expect([
      dayLabel(new Date(2026, 9, 1, 0, 5), now),
      dayLabel(new Date(2026, 8, 30, 23, 55), now),
    ]).toEqual(['Today', 'Yesterday']);
  });

  it('dates older days', () => {
    expect(dayLabel(new Date(2026, 8, 28, 12, 0), now)).not.toMatch(/Today|Yesterday/);
  });
});

describe('groupByDay', () => {
  it('splits entries at local midnight, keeping order', () => {
    const days = groupByDay(
      [
        entry(new Date(2026, 9, 1, 14, 0)),
        entry(new Date(2026, 9, 1, 9, 0)),
        entry(new Date(2026, 8, 30, 22, 0)),
      ],
      now
    );
    expect(days.map((d) => [d.label, d.entries.length])).toEqual([
      ['Today', 2],
      ['Yesterday', 1],
    ]);
  });
});

describe('relativeTime', () => {
  it('counts minutes and hours within a day', () => {
    expect([
      relativeTime(new Date(2026, 9, 1, 14, 59, 30), now),
      relativeTime(new Date(2026, 9, 1, 14, 55), now),
      relativeTime(new Date(2026, 9, 1, 12, 0), now),
    ]).toEqual(['just now', '5m ago', '3h ago']);
  });
});
