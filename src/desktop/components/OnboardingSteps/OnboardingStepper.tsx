import type { FlowState } from './onboarding-flow';
import { STEP_LABELS, stepStatus } from './onboarding-flow';
import AnimatedCheck from './AnimatedCheck';

interface OnboardingStepperProps {
  flow: FlowState;
  /** Short caption under the current step, e.g. "Installing…". */
  caption: string;
}

export default function OnboardingStepper({ flow, caption }: OnboardingStepperProps) {
  return (
    <ol aria-label="Setup steps" className="m-0 flex list-none flex-col p-0">
      {STEP_LABELS.map((label, i) => {
        const status = stepStatus(flow, i);
        const last = i === STEP_LABELS.length - 1;
        return (
          <li
            key={label}
            className="flex gap-3.5"
            aria-current={status === 'current' ? 'step' : undefined}
          >
            <div className="flex flex-col items-center">
              <span
                className={`ob-node flex h-[30px] w-[30px] flex-none items-center justify-center rounded-full border text-sm font-bold ${
                  status === 'done'
                    ? 'border-ok bg-ok text-on-pri'
                    : status === 'current'
                      ? 'border-pri bg-pri text-on-pri'
                      : 'border-line-2 bg-transparent text-ink-2'
                }`}
                data-state={status}
              >
                <span className="ob-node-num">{i + 1}</span>
                <span className="ob-node-check">
                  <AnimatedCheck drawn={status === 'done'} />
                </span>
              </span>
              {!last && (
                <span className="relative h-9 w-0.5 bg-line-soft">
                  <span
                    className="ob-connector absolute inset-0 bg-ok"
                    data-filled={String(status === 'done')}
                  />
                </span>
              )}
            </div>
            <div className="pt-[5px]">
              <div
                className={`text-[15px] ${status === 'current' ? 'font-bold text-ink' : 'font-semibold'} ${
                  status === 'pending' ? 'text-ink-2' : 'text-ink'
                }`}
              >
                {label}
              </div>
              {status !== 'pending' && (
                <div className="text-xs text-ink-2">{status === 'done' ? 'Done' : caption}</div>
              )}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
