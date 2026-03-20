import { useState, useEffect } from 'react';
import Card from '../../../components/Card';
import { tauriBridge } from '../../api/tauri-bridge';

const THEMES = [
  { id: 'light', label: 'Light' },
  { id: 'dark', label: 'Dark' },
  { id: 'system', label: 'System' },
] as const;

export default function AppearanceSettings() {
  const [theme, setTheme] = useState('system');

  useEffect(() => {
    tauriBridge.getPreferredTheme().then(setTheme);
  }, []);

  const handleChange = async (value: string) => {
    setTheme(value);
    await tauriBridge.setPreferredTheme(value);
    // Apply to shell immediately
    // Resolve to CLI theme values: clean (light), dark
    const resolved =
      value === 'system'
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'clean'
        : value === 'light'
          ? 'clean'
          : value;
    document.documentElement.setAttribute('data-theme', resolved);
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        Appearance
      </h1>

      <Card>
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-pencil">Theme</p>
            <p className="text-xs text-pencil-light mt-0.5">
              Controls the app and CLI UI appearance
            </p>
          </div>
          <div className="flex rounded-[var(--radius-sm)] border border-muted overflow-hidden">
            {THEMES.map(({ id, label }) => (
              <button
                key={id}
                type="button"
                onClick={() => handleChange(id)}
                className={`px-3 py-1.5 text-sm transition-colors ${
                  theme === id
                    ? 'bg-pencil text-paper font-medium'
                    : 'bg-paper text-pencil-light hover:text-pencil'
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </Card>
    </div>
  );
}
