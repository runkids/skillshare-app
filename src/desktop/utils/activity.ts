import type { ActivityEntry } from '../api/tauri-bridge';

export interface ActivityDay {
  label: string;
  entries: ActivityEntry[];
}

const DAY_MS = 86_400_000;

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** "Today", "Yesterday", or the local date for older days. */
export function dayLabel(date: Date, now: Date): string {
  // Round, because a day that crosses a DST change is 23 or 25 hours long.
  const days = Math.round((startOfDay(now) - startOfDay(date)) / DAY_MS);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  return date.toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric' });
}

/** Group entries (already newest first) into consecutive local days. */
export function groupByDay(entries: ActivityEntry[], now: Date): ActivityDay[] {
  const days: ActivityDay[] = [];
  for (const entry of entries) {
    const label = dayLabel(new Date(entry.ts), now);
    const last = days[days.length - 1];
    if (last?.label === label) last.entries.push(entry);
    else days.push({ label, entries: [entry] });
  }
  return days;
}

/** "just now", "5m ago", "3h ago", then the clock time once a day has passed. */
export function relativeTime(date: Date, now: Date): string {
  const seconds = Math.max(0, Math.round((now.getTime() - date.getTime()) / 1000));
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}
