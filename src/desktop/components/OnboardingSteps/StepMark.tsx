import AnimatedCheck from './AnimatedCheck';

export type MarkState = 'pending' | 'active' | 'done';

/** Row marker for the install step list: dashed ring, spinner, then a drawn check. */
export default function StepMark({ state }: { state: MarkState }) {
  return (
    <span className="ob-mark" data-state={state} aria-hidden="true">
      {state === 'active' && <span className="ob-spin" />}
      {state === 'done' && <AnimatedCheck drawn />}
    </span>
  );
}
