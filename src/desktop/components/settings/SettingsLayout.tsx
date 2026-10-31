import type { ReactNode } from 'react';

/** Page title for a settings tab: the shared h1 token and an optional action. */
export function SettingsHeader({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <header className="flex items-center gap-4">
      <h1
        className="m-0 flex-1 min-w-0 text-[var(--ink)]"
        style={{ font: 'var(--h1)', letterSpacing: 'var(--h1-track)' }}
      >
        {title}
      </h1>
      {action}
    </header>
  );
}

/** Update progress. With `value` (0-100) it fills to that point; without, a band sweeps the track. */
export function ProgressBar({ label, value }: { label: string; value?: number }) {
  const known = value !== undefined;
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={known ? value : undefined}
      className="relative h-2 rounded-full bg-[var(--sunken)] overflow-hidden"
    >
      <div
        className={`progress-fill absolute inset-y-0 left-0 rounded-full ${known ? 'transition-[width] duration-300' : 'progress-sweep'}`}
        style={{ width: known ? `${value}%` : '35%' }}
      />
    </div>
  );
}

/** A titled group of settings. `heading` "h2" uses the theme's h2 token; "label" is a small caption. */
export function SettingsSection({
  title,
  heading = 'label',
  action,
  children,
}: {
  title: ReactNode;
  heading?: 'h2' | 'label';
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2.5">
      <div className="flex items-center gap-3">
        <h2
          className={`m-0 flex-1 ${heading === 'label' ? 'text-[13px] font-semibold text-[var(--ink-2)]' : 'text-[var(--ink)]'}`}
          style={heading === 'h2' ? { font: 'var(--h2)', letterSpacing: 'var(--h2-track)' } : {}}
        >
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

/** One framed list; its rows are separated by the theme's separator. */
export function SettingsList({
  children,
  className = '',
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`bg-[var(--surface)] rounded-[var(--r-box)] shadow-[var(--sh-box)] [&>*+*]:[border-top:var(--sep)] ${className}`}
      style={{ border: 'var(--frame)' }}
    >
      {children}
    </div>
  );
}

/** A label + hint on the left and a control on the right. */
export function SettingsRow({
  label,
  hint,
  id,
  children,
}: {
  label: ReactNode;
  hint?: ReactNode;
  id?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex items-center gap-3.5 min-h-14 px-4 py-3">
      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        <span id={id} className="text-sm font-semibold text-[var(--ink)]">
          {label}
        </span>
        {hint && (
          <span id={id ? `${id}-hint` : undefined} className="text-[13px] text-[var(--ink-2)]">
            {hint}
          </span>
        )}
      </div>
      {children}
    </div>
  );
}

/** Status as a colored dot plus a word, never color alone. */
export function StatusDot({
  tone,
  children,
}: {
  tone: 'ok' | 'warn' | 'bad' | 'accent' | 'muted';
  children: ReactNode;
}) {
  const color = {
    ok: 'text-[var(--ok)]',
    warn: 'text-[var(--warn)]',
    bad: 'text-[var(--bad)]',
    accent: 'text-[var(--accent)]',
    muted: 'text-[var(--ink-2)]',
  }[tone];
  return (
    <span
      className={`inline-flex items-center gap-1.5 text-[13px] font-medium whitespace-nowrap ${color}`}
    >
      <i className="w-[7px] h-[7px] rounded-full bg-current" aria-hidden="true" />
      {children}
    </span>
  );
}
