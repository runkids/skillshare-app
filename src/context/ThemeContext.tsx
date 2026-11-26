import { useState, useEffect, useCallback, useMemo, useSyncExternalStore } from 'react';
import type { ReactNode } from 'react';
import { ThemeContext, type Style, type ModePreference } from './useTheme';
import {
  cssVars,
  customAttributes,
  loadCustomTheme,
  storeCustomTheme,
  type SavedCustomTheme,
} from './customTheme';

function getInitialStyle(): Style {
  const stored = localStorage.getItem('skillshare-style');
  if (stored === 'playful') return 'playful';
  // Custom needs a saved theme; without one the app falls back to Clean.
  if (stored === 'custom' && loadCustomTheme()) return 'custom';
  return 'clean';
}

function getInitialModePreference(): ModePreference {
  // Migration: old 'skillshare-theme' key → new 'skillshare-theme-preference'
  const legacy = localStorage.getItem('skillshare-theme');
  if (legacy === 'light' || legacy === 'dark') {
    localStorage.setItem('skillshare-theme-preference', legacy);
    localStorage.removeItem('skillshare-theme');
    return legacy;
  }

  const stored = localStorage.getItem('skillshare-theme-preference');
  if (stored === 'light' || stored === 'dark' || stored === 'system') return stored;
  return 'light';
}

function resolveMode(pref: ModePreference): 'light' | 'dark' {
  if (pref === 'light') return 'light';
  if (pref === 'dark') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function getSystemMode() {
  return resolveMode('system');
}

function subscribeToSystemMode(onChange: () => void) {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  mq.addEventListener('change', onChange);
  return () => mq.removeEventListener('change', onChange);
}

let transitionTimer: ReturnType<typeof setTimeout>;
function applyWithTransition(fn: () => void) {
  clearTimeout(transitionTimer);
  document.documentElement.classList.add('theme-transitioning');
  fn();
  transitionTimer = setTimeout(() => {
    document.documentElement.classList.remove('theme-transitioning');
  }, 300);
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [style, setStyleState] = useState<Style>(getInitialStyle);
  const [customTheme, setCustomThemeState] = useState<SavedCustomTheme | null>(loadCustomTheme);
  const [modePreference, setModePreferenceState] =
    useState<ModePreference>(getInitialModePreference);
  const systemMode = useSyncExternalStore(subscribeToSystemMode, getSystemMode);
  const resolvedMode = modePreference === 'system' ? systemMode : modePreference;

  // Apply style to DOM. A custom theme takes its base style's shapes and its own colours.
  useEffect(() => {
    const root = document.documentElement;
    const active = style === 'custom' ? customTheme : null;
    const custom = active && customAttributes(active);
    if (style === 'playful' || custom?.playful) {
      root.setAttribute('data-theme', 'playful');
    } else {
      root.removeAttribute('data-theme');
    }
    if (custom?.full) root.setAttribute('data-layout', 'full');
    else root.removeAttribute('data-layout');
    const vars = cssVars(active?.vars[resolvedMode] ?? {});
    for (const [key, value] of Object.entries(vars)) root.style.setProperty(key, value);
    localStorage.setItem('skillshare-style', style);
    return () => {
      for (const key of Object.keys(vars)) root.style.removeProperty(key);
    };
  }, [style, customTheme, resolvedMode]);

  // Apply mode to DOM and handle system listener
  useEffect(() => {
    const root = document.documentElement;
    const resolved = resolvedMode;

    if (resolved === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }

    localStorage.setItem('skillshare-theme-preference', modePreference);
  }, [modePreference, resolvedMode]);

  const setStyle = useCallback((s: Style) => {
    applyWithTransition(() => setStyleState(s));
  }, []);

  const setCustomTheme = useCallback((t: SavedCustomTheme) => {
    storeCustomTheme(t);
    setCustomThemeState(t);
    setStyleState('custom');
  }, []);

  const setModePreference = useCallback((m: ModePreference) => {
    applyWithTransition(() => setModePreferenceState(m));
  }, []);

  const toggleTheme = useCallback(() => {
    setModePreference(resolvedMode === 'light' ? 'dark' : 'light');
  }, [resolvedMode, setModePreference]);

  const value = useMemo(
    () => ({
      style,
      setStyle,
      customTheme,
      setCustomTheme,
      modePreference,
      setModePreference,
      resolvedMode,
      theme: resolvedMode,
      toggleTheme,
    }),
    [
      style,
      setStyle,
      customTheme,
      setCustomTheme,
      modePreference,
      setModePreference,
      resolvedMode,
      toggleTheme,
    ]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
