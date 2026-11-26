import { createContext, useContext } from 'react';
import type { SavedCustomTheme } from './customTheme';

export type Style = 'clean' | 'playful' | 'custom';
export type ModePreference = 'light' | 'dark' | 'system';
export type ResolvedMode = 'light' | 'dark';

interface ThemeContextValue {
  style: Style;
  setStyle: (s: Style) => void;
  /** The custom theme last made in the CLI UI, if any. */
  customTheme: SavedCustomTheme | null;
  /** Keep a custom theme the CLI UI reported and switch to it. */
  setCustomTheme: (t: SavedCustomTheme) => void;
  modePreference: ModePreference;
  setModePreference: (m: ModePreference) => void;
  resolvedMode: ResolvedMode;
  // Legacy compat
  theme: ResolvedMode;
  toggleTheme: () => void;
}

export const ThemeContext = createContext<ThemeContextValue>({
  style: 'clean',
  setStyle: () => {},
  customTheme: null,
  setCustomTheme: () => {},
  modePreference: 'light',
  setModePreference: () => {},
  resolvedMode: 'light',
  theme: 'light',
  toggleTheme: () => {},
});

export function useTheme() {
  return useContext(ThemeContext);
}
