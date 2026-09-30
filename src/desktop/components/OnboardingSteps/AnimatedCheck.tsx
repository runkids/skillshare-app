import { useEffect, useState } from 'react';

interface AnimatedCheckProps {
  size?: number;
  /** When set, the stroke draws in once this turns true. Omit to let a parent's CSS drive it. */
  drawn?: boolean;
}

export default function AnimatedCheck({ size = 15, drawn }: AnimatedCheckProps) {
  // Render undrawn first so the stroke animates in instead of appearing pre-drawn.
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setReady(true));
    return () => cancelAnimationFrame(id);
  }, []);

  return (
    <svg
      className="ob-check"
      data-drawn={drawn === undefined ? undefined : String(drawn && ready)}
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M5 12.5l4.5 4.5L19 7.5" pathLength={1} />
    </svg>
  );
}
