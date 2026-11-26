// A custom theme made in the CLI UI's theme editor (Settings → Appearance). The CLI UI derives
// its colours and reports them with each `theme-change`; the app keeps the theme to hand back
// (WebKit drops the iframe's own storage) and paints its own window with those colours.

import type { ResolvedMode } from './useTheme';

export interface SavedCustomTheme {
  /** The theme as the CLI UI reported it. The CLI UI validates it again when it gets it back. */
  custom: Record<string, unknown> & { base: 'clean' | 'playful'; layout?: string };
  /** Custom properties for each mode, without the leading `--`. */
  vars: Record<ResolvedMode, Record<string, string>>;
}

const CUSTOM_THEME_KEY = 'skillshare-custom-theme';

// The tokens a custom theme sets (the CLI UI's `themeVars`). Anything else is ignored, so a
// message cannot restyle properties the theme does not own.
const KEYS = new Set([
  'bg',
  'side',
  'surface',
  'sunken',
  'shell',
  'ink',
  'ink-2',
  'ink-3',
  'sel',
  'sel-ink',
  'line',
  'line-2',
  'line-soft',
  'sep',
  'frame',
  'glass-bar',
  'glass-menu',
  'pri',
  'on-pri',
  'accent',
  'accent-bg',
  'ok',
  'ok-bg',
  'warn',
  'warn-bg',
  'bad',
  'bad-bg',
  'r-ctl',
  'r-btn',
  'r-box',
  'r-tag',
  'h1',
  'h1-track',
  // A font stack. The app does not load the web font itself: an installed copy is used, and the
  // system font otherwise.
  'f',
  'fm',
]);
// Colours, lengths, borders, font stacks and the heading shorthand. rgba() and var() are the only
// functions, so no url() can fetch anything.
const VALUE = /^[#\w.%/\s',()-]+$/;

function readVars(input: unknown): Record<string, string> | null {
  if (!input || typeof input !== 'object' || Array.isArray(input)) return null;
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(input)) {
    if (!KEYS.has(key)) continue;
    if (typeof value !== 'string' || value.length > 200 || !VALUE.test(value)) return null;
    if (/\(/.test(value.replace(/rgba?\(|var\(--/g, ''))) return null;
    out[key] = value;
  }
  return out;
}

/** The theme and colours from a `theme-change`, or null when they are not usable. */
export function readCustomTheme(custom: unknown, vars: unknown): SavedCustomTheme | null {
  if (!custom || typeof custom !== 'object' || Array.isArray(custom)) return null;
  const theme = custom as Record<string, unknown>;
  if (theme.v !== 1 || (theme.base !== 'clean' && theme.base !== 'playful')) return null;
  if (JSON.stringify(theme).length > 2000) return null;
  if (!vars || typeof vars !== 'object') return null;
  const light = readVars((vars as Record<string, unknown>).light);
  const dark = readVars((vars as Record<string, unknown>).dark);
  if (!light || !dark) return null;
  return { custom: theme as SavedCustomTheme['custom'], vars: { light, dark } };
}

export function loadCustomTheme(): SavedCustomTheme | null {
  try {
    const saved = JSON.parse(localStorage.getItem(CUSTOM_THEME_KEY) ?? 'null');
    return saved ? readCustomTheme(saved.custom, saved.vars) : null;
  } catch {
    return null;
  }
}

export function storeCustomTheme(theme: SavedCustomTheme) {
  try {
    localStorage.setItem(CUSTOM_THEME_KEY, JSON.stringify(theme));
  } catch {
    // The theme still applies until the app quits.
  }
}

/** Whether two reports describe the same theme. */
export function sameCustomTheme(a: SavedCustomTheme | null, b: SavedCustomTheme | null) {
  return !!a && !!b && JSON.stringify(a.custom) === JSON.stringify(b.custom);
}

/** base64url of the theme's JSON, as the CLI UI reads `?custom-theme=`. */
export function encodeCustomTheme(theme: SavedCustomTheme): string {
  let bin = '';
  new TextEncoder().encode(JSON.stringify(theme.custom)).forEach((b) => {
    bin += String.fromCharCode(b);
  });
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** The theme's colours for one mode as CSS custom properties, `--` included. */
export function cssVars(vars: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(vars).map(([key, value]) => [`--${key}`, value]));
}

/** The <html> attributes for a theme, as the CLI UI sets them: the base style, and Clean's
 *  inset panel turned off. */
export function customAttributes(theme: SavedCustomTheme) {
  const { base, layout } = theme.custom;
  return {
    playful: base === 'playful',
    full: base === 'clean' && layout === 'full',
  };
}
