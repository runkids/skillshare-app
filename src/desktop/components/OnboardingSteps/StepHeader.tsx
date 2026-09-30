import type { ReactNode } from 'react';

interface StepHeaderProps {
  step: number;
  title: string;
  children?: ReactNode;
}

export default function StepHeader({ step, title, children }: StepHeaderProps) {
  return (
    <>
      <div className="text-xs font-bold tracking-[0.08em] text-ink-3">STEP {step} OF 3</div>
      <h1
        className="m-0 text-[28px] font-bold leading-[1.1] tracking-[-0.022em] text-ink"
        style={{ fontFamily: 'var(--font-heading)' }}
      >
        {title}
      </h1>
      {children && (
        <p className="m-0 max-w-[520px] text-[15px] leading-[1.55] text-ink-2">{children}</p>
      )}
    </>
  );
}
