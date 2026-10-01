import { useId } from 'react';
import type { InputHTMLAttributes } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export default function Input({ label, className = '', style, id, ...props }: InputProps) {
  const autoId = useId();
  const inputId = id ?? autoId;

  return (
    <div>
      {label && (
        <label htmlFor={inputId} className="block text-base text-pencil-light mb-1">
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`
          ss-input
          w-full px-4 py-2.5 bg-surface border-[length:var(--bw)] border-[var(--line-2)] text-pencil
          placeholder:text-muted-dark
          hover:border-muted-dark
          focus:outline-none focus:border-pencil
          transition-all
          rounded-[var(--r-ctl)]
          ${className}
        `}
        style={{
          fontSize: '1rem',
          ...style,
        }}
        {...props}
      />
    </div>
  );
}
