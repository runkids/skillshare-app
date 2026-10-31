import type { ReactNode } from 'react';

interface StepHeaderProps {
  step: number;
  title: string;
  children?: ReactNode;
}

export default function StepHeader({ step, title, children }: StepHeaderProps) {
  return (
    <>
      <div className="text-[12.5px] font-semibold text-ink-3">Step {step} of 3</div>
      <h1 className="m-0 text-ink" style={{ font: 'var(--h1)', letterSpacing: 'var(--h1-track)' }}>
        {title}
      </h1>
      {children && (
        <p className="m-0 max-w-[520px] text-[15px] leading-[1.55] text-ink-2">{children}</p>
      )}
    </>
  );
}
