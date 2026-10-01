import type { ITheme, ITerminalOptions } from '@xterm/xterm';

export const terminalTheme: ITheme = {
  background: '#030712', // gray-950
  foreground: '#e5e7eb', // gray-200
  cursor: '#e5e7eb',
  cursorAccent: '#030712',
  selectionBackground: '#374151', // gray-700
  selectionForeground: '#f9fafb', // gray-50
  black: '#1f2937',
  red: '#ef4444',
  green: '#22c55e',
  yellow: '#eab308',
  blue: '#3b82f6',
  magenta: '#a855f7',
  cyan: '#06b6d4',
  white: '#e5e7eb',
  brightBlack: '#6b7280',
  brightRed: '#f87171',
  brightGreen: '#4ade80',
  brightYellow: '#facc15',
  brightBlue: '#60a5fa',
  brightMagenta: '#c084fc',
  brightCyan: '#22d3ee',
  brightWhite: '#f9fafb',
};

export const terminalOptions: ITerminalOptions = {
  theme: terminalTheme,
  fontFamily: '"SF Mono", Menlo, Monaco, "Cascadia Code", "Fira Code", "Courier New", monospace',
  fontSize: 13,
  lineHeight: 1.4,
  cursorBlink: true,
  cursorStyle: 'block',
  scrollback: 5000,
  allowProposedApi: true,
};
