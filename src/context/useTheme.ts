import { createContext, useContext } from 'react';

export type Style = 'clean' | 'playful';
export type ModePreference = 'light' | 'dark' | 'system';
export type ResolvedMode = 'light' | 'dark';

interface ThemeContextValue {
  style: Style;
  setStyle: (s: Style) => void;
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
  modePreference: 'light',
  setModePreference: () => {},
  resolvedMode: 'light',
  theme: 'light',
  toggleTheme: () => {},
});

export function useTheme() {
  return useContext(ThemeContext);
}
