import type { ReactNode, ButtonHTMLAttributes, Ref } from 'react';
import Spinner from './Spinner';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  ref?: Ref<HTMLButtonElement>;
}

const variantClasses = {
  primary:
    'bg-[var(--pri)] text-[var(--on-pri)] border-[length:var(--bw)] border-[var(--pri)] hover:opacity-85',
  secondary:
    'bg-surface text-pencil border-[length:var(--bw)] border-[var(--line-2)] hover:bg-muted/30 hover:border-pencil hover:shadow-sm',
  danger:
    'bg-transparent text-danger border-[length:var(--bw)] border-danger hover:bg-danger hover:text-white',
  ghost: 'bg-transparent text-pencil-light hover:text-pencil hover:bg-muted/30',
  link: 'bg-transparent text-pencil-light hover:text-pencil hover:underline border-none',
};

const sizeClasses = {
  sm: 'h-7 px-3 text-[12.5px]',
  md: 'h-[34px] px-3.5 text-[13px]',
  lg: 'h-[42px] px-5 text-sm',
};

export default function Button({
  children,
  variant = 'primary',
  size = 'md',
  className = '',
  disabled,
  loading = false,
  style,
  ref,
  ...props
}: ButtonProps) {
  const isLink = variant === 'link';
  const isGhostOrLink = variant === 'ghost' || variant === 'link';
  const isDisabled = disabled || loading;
  return (
    <button
      ref={ref}
      className={`
        ${isGhostOrLink ? '' : 'ss-btn'}
        inline-flex items-center justify-center gap-2
        font-medium
        transition-all duration-150 cursor-pointer
        active:scale-[0.98]
        focus-visible:ring-2 focus-visible:ring-pencil/20 focus-visible:ring-offset-2
        disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100
        ${variantClasses[variant]}
        ${isLink ? 'text-sm p-0' : `${sizeClasses[size]} rounded-[var(--radius-btn)]`}
        ${className}
      `}
      style={{ boxShadow: isGhostOrLink ? undefined : 'var(--sh-btn)', ...style }}
      disabled={isDisabled}
      {...props}
    >
      {loading && <Spinner size="sm" className="text-current" />}
      {children}
    </button>
  );
}
