import { radius } from '../design';

interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'card' | 'circle';
  style?: React.CSSProperties;
}

export default function Skeleton({ className = '', variant = 'text', style }: SkeletonProps) {
  const base = 'animate-skeleton';

  if (variant === 'circle') {
    return (
      <div
        className={`${base} w-12 h-12 ${className}`}
        style={{ borderRadius: '50%', ...style }}
      />
    );
  }

  if (variant === 'card') {
    return (
      <div
        className={`${base} border border-muted p-4 h-32 ${className}`}
        style={{ borderRadius: radius.md, ...style }}
      />
    );
  }

  return (
    <div
      className={`${base} h-4 ${className}`}
      style={{ borderRadius: radius.sm, ...style }}
    />
  );
}

/** A full loading skeleton for a page */
export function PageSkeleton() {
  return (
    <div className="space-y-6 animate-fade-in">
      <Skeleton className="w-48 h-8" />
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {[0, 1, 2].map((i) => (
          <Skeleton
            key={i}
            variant="card"
            className="animate-skeleton"
            style={{ animationDelay: `${i * 50}ms` } as React.CSSProperties}
          />
        ))}
      </div>
      {[0, 1, 2].map((i) => (
        <Skeleton
          key={i}
          className={i === 0 ? 'w-full h-4' : i === 1 ? 'w-3/4 h-4' : 'w-1/2 h-4'}
          style={{ animationDelay: `${(i + 3) * 50}ms` } as React.CSSProperties}
        />
      ))}
    </div>
  );
}

/** Custom skeleton for the Skill Detail page */
export function SkillDetailSkeleton() {
  return (
    <div className="animate-fade-in">
      {/* Header: back button + title + badges */}
      <div className="flex items-center gap-3 mb-6 py-3">
        <Skeleton className="w-10 h-10 shrink-0" style={{ borderRadius: radius.md }} />
        <div className="flex items-center gap-3">
          <Skeleton className="w-48 h-8" />
          <Skeleton className="w-20 h-6" style={{ borderRadius: radius.full }} />
          <Skeleton className="w-16 h-6" style={{ borderRadius: radius.full }} />
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main content card */}
        <div className="lg:col-span-2">
          <div className="border border-muted p-6 space-y-4" style={{ borderRadius: radius.md }}>
            {/* Manifest block */}
            <div className="p-4 border border-muted space-y-3" style={{ borderRadius: radius.sm }}>
              <Skeleton className="w-32 h-3" />
              <Skeleton className="w-40 h-6" />
              <Skeleton className="w-full h-4" />
            </div>
            {/* Stats bar */}
            <div className="flex gap-4 py-3 border-b border-muted">
              <Skeleton className="w-24 h-4" />
              <Skeleton className="w-20 h-4" />
              <Skeleton className="w-16 h-4" />
            </div>
            {/* Markdown content lines */}
            <div className="space-y-3 pt-2">
              <Skeleton className="w-2/5 h-6" />
              <Skeleton className="w-full h-4" />
              <Skeleton className="w-full h-4" style={{ animationDelay: '50ms' }} />
              <Skeleton className="w-4/5 h-4" style={{ animationDelay: '100ms' }} />
              <Skeleton className="w-full h-4" style={{ animationDelay: '150ms' }} />
              <Skeleton className="w-3/5 h-4" style={{ animationDelay: '200ms' }} />
              <div className="pt-2" />
              <Skeleton className="w-1/3 h-6" style={{ animationDelay: '250ms' }} />
              <Skeleton className="w-full h-4" style={{ animationDelay: '300ms' }} />
              <Skeleton className="w-5/6 h-4" style={{ animationDelay: '350ms' }} />
              <Skeleton className="w-2/3 h-4" style={{ animationDelay: '400ms' }} />
            </div>
          </div>
        </div>

        {/* Sidebar cards */}
        <div className="space-y-5">
          {/* Metadata card */}
          <div className="border border-muted p-5 space-y-3" style={{ borderRadius: radius.md }}>
            <Skeleton className="w-24 h-5" />
            <div className="space-y-3">
              <div><Skeleton className="w-12 h-3" /><Skeleton className="w-full h-4 mt-1" /></div>
              <div><Skeleton className="w-16 h-3" /><Skeleton className="w-3/4 h-4 mt-1" /></div>
              <div><Skeleton className="w-20 h-3" /><Skeleton className="w-1/2 h-4 mt-1" /></div>
            </div>
            <div className="flex gap-2 pt-3 border-t border-muted">
              <Skeleton className="flex-1 h-9" style={{ borderRadius: radius.btn }} />
              <Skeleton className="flex-1 h-9" style={{ borderRadius: radius.btn }} />
            </div>
          </div>

          {/* Security card */}
          <div className="border border-muted p-5 space-y-3" style={{ borderRadius: radius.md }}>
            <Skeleton className="w-20 h-5" />
            <div className="flex gap-2">
              <Skeleton className="w-24 h-8" style={{ borderRadius: radius.sm }} />
              <Skeleton className="w-32 h-8" style={{ borderRadius: radius.sm }} />
            </div>
          </div>

          {/* Files card */}
          <div className="border border-muted p-5 space-y-3" style={{ borderRadius: radius.md }}>
            <Skeleton className="w-28 h-5" />
            {[0, 1, 2, 3].map((i) => (
              <div key={i} className="flex items-center gap-2">
                <Skeleton className="w-4 h-4 shrink-0" />
                <Skeleton className={`h-4 ${i === 0 ? 'w-24' : i === 1 ? 'w-32' : i === 2 ? 'w-28' : 'w-20'}`} />
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
