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
      className="flex items-start gap-2.5 rounded-[var(--r-ctl)] bg-[#1E1D1B] px-3.5 py-3 text-[13px] text-[#F7F6F3]"
      style={{ fontFamily: 'var(--font-mono)' }}
    >
      <span className="text-[#A8A49B]">$</span>
      <code className="min-w-0 flex-1 break-all">{command}</code>
      <button
        type="button"
        aria-label={state === 'failed' ? 'Copy failed' : 'Copy command'}
        onClick={copy}
        className="flex h-[30px] w-[30px] flex-none cursor-pointer items-center justify-center rounded-[var(--r-ctl)] border border-white/25 bg-transparent text-[#F7F6F3] hover:bg-white/10 focus-visible:ring-2 focus-visible:ring-white/50"
      >
        {state === 'copied' ? <Check size={15} /> : <Copy size={15} />}
      </button>
    </div>
  );
}
