/**
 * Minimal Design System Constants
 *
 * Subtle shadows for inline styles where Tailwind classes aren't sufficient.
 */

/** Shadow presets (mirrors CSS variables for inline use) */
export const shadows = {
  sm: 'var(--shadow-sm)',
  md: 'var(--shadow-md)',
  lg: 'var(--shadow-lg)',
  hover: 'var(--shadow-hover)',
  active: 'none',
  accent: 'var(--shadow-accent)',
  blue: 'var(--shadow-blue)',
} as const;
