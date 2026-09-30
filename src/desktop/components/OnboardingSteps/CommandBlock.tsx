import { useState } from 'react';
import { Check, Copy } from 'lucide-react';

export default function CommandBlock({ command }: { command: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setState('copied');
    } catch {
      setState('failed');
    }
    setTimeout(() => setState('idle'), 1500);
  };

  return (
    <div
      className="flex items-start gap-2.5 rounded-lg bg-ink px-3.5 py-3 text-[13px] text-bg"
      style={{ fontFamily: 'var(--font-mono)' }}
    >
      <span className="text-link-bg">$</span>
      <code className="min-w-0 flex-1 break-all">{command}</code>
      <button
        type="button"
        aria-label={state === 'failed' ? 'Copy failed' : 'Copy command'}
        onClick={copy}
        className="flex h-[30px] w-[30px] flex-none cursor-pointer items-center justify-center rounded-lg border border-bg/30 bg-transparent text-bg hover:bg-bg/10 focus-visible:ring-2 focus-visible:ring-bg/50"
      >
        {state === 'copied' ? <Check size={15} /> : <Copy size={15} />}
      </button>
    </div>
  );
}
