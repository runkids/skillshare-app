import type { CSSProperties } from 'react';
import { ArrowRight, Monitor, Moon, Sun } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useTheme, type Style, type ModePreference } from '../../../context/useTheme';
import { cssVars } from '../../../context/customTheme';
import { isMacOS } from '../../utils/platform';

const STYLES: { id: Style; label: string; description: string }[] = [
  { id: 'clean', label: 'Clean', description: 'Quiet, system type, thin lines' },
  { id: 'playful', label: 'Playful', description: 'Hand-drawn headings, ink outlines' },
];

const MODES: { id: ModePreference; label: string; icon: typeof Sun }[] = [
  { id: 'light', label: 'Light', icon: Sun },
  { id: 'dark', label: 'Dark', icon: Moon },
  { id: 'system', label: 'System', icon: Monitor },
];

// Mini previews are drawn from the active tokens; the shapes (thin lines vs. bold
// outlines, hard shadow, hand-drawn type) carry the difference between styles.
function StylePreview({ id }: { id: Exclude<Style, 'custom'> }) {
  if (id === 'clean') {
    return (
      <div className="h-[132px] flex gap-2.5 p-4 bg-[var(--bg)] border-b border-[var(--line)]">
        <div className="w-[60px] flex flex-col gap-1.5">
          <span className="h-2 w-10 rounded bg-[var(--ink)]" />
          <span className="h-3.5 rounded-[5px] bg-[var(--sel)]" />
          <span className="h-2 rounded bg-[var(--line-2)]" />
          <span className="h-2 rounded bg-[var(--line-2)]" />
        </div>
        <div className="flex-1 flex flex-col gap-2">
          <span className="h-3 w-[90px] rounded bg-[var(--ink)]" />
          <span className="flex-1 rounded-lg bg-[var(--surface)] border border-[var(--line)]" />
        </div>
      </div>
    );
  }
  return (
    <div className="h-[132px] flex gap-2.5 p-4 bg-[var(--sunken)] border-b border-[var(--line)]">
      <div className="w-[60px] flex flex-col gap-1.5">
        <span className="text-xs leading-none text-[var(--ink)] font-['Kalam',cursive]">
          skillshare
        </span>
        <span className="h-3.5 rounded-[5px] bg-[var(--sel)] border-[1.5px] border-[var(--ink)]" />
        <span className="h-2 rounded bg-[var(--line-2)]" />
        <span className="h-2 rounded bg-[var(--line-2)]" />
      </div>
      <div className="flex-1 flex flex-col gap-2">
        <span className="text-[15px] leading-none text-[var(--ink)] font-['Kalam',cursive]">
          Dashboard
        </span>
        <span className="flex-1 rounded-[10px] bg-[var(--surface)] border-[1.5px] border-[var(--ink)] shadow-[3px_3px_0_var(--ink)]" />
      </div>
    </div>
  );
}

export default function AppearanceSettings() {
  const navigate = useNavigate();
  const { style, setStyle, customTheme, resolvedMode, modePreference, setModePreference } =
    useTheme();
  // The custom theme is made in the dashboard; here it is one more style to pick, previewed in
  // its own colours.
  const styles = customTheme
    ? [
        ...STYLES,
        {
          id: 'custom' as const,
          label:
            typeof customTheme.custom.name === 'string' && customTheme.custom.name
              ? customTheme.custom.name
              : 'Custom',
          description: 'Your theme from the dashboard',
        },
      ]
    : STYLES;
  const customVars = customTheme
    ? (cssVars(customTheme.vars[resolvedMode]) as CSSProperties)
    : undefined;

  const handleStyleChange = (s: Style) => {
    setStyle(s);
    // Style and mode are persisted in localStorage by ThemeContext.
  };

  const handleModeChange = (m: ModePreference) => {
    setModePreference(m);
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1
          className="text-[var(--ink)]"
          style={{ font: 'var(--h1)', letterSpacing: 'var(--h1-track)' }}
        >
          Appearance
        </h1>
      </header>

      <section className="flex flex-col gap-3">
        <h2 className="text-[15px] font-bold text-[var(--ink)]">Style</h2>
        <div
          role="radiogroup"
          aria-label="Style"
          className={`grid gap-4 ${customTheme ? 'grid-cols-3' : 'grid-cols-2'}`}
        >
          {styles.map(({ id, label, description }) => {
            const checked = style === id;
            return (
              <button
                key={id}
                type="button"
                role="radio"
                aria-checked={checked}
                onClick={() => handleStyleChange(id)}
                className={`text-left overflow-hidden rounded-[var(--r-box)] bg-[var(--surface)] border-2 transition-[border-color,box-shadow] ${
                  checked
                    ? 'border-[var(--accent)] shadow-[0_0_0_4px_var(--accent-bg)]'
                    : 'border-[var(--line)] hover:border-[var(--line-2)]'
                }`}
              >
                {id === 'custom' ? (
                  <span className="block" style={customVars}>
                    <StylePreview id={customTheme?.custom.base ?? 'clean'} />
                  </span>
                ) : (
                  <StylePreview id={id} />
                )}
                <span className="flex items-center gap-2.5 px-4 py-3.5">
                  <span
                    className={`w-[18px] h-[18px] shrink-0 rounded-full bg-[var(--surface)] ${
                      checked
                        ? 'border-[5px] border-[var(--accent)]'
                        : 'border-2 border-[var(--line-2)]'
                    }`}
                  />
                  <span>
                    <span className="block text-sm font-bold text-[var(--ink)]">
                      {label}
                      {id === 'clean' && (
                        <span className="ml-1 px-1.5 py-px rounded-[var(--r-tag)] bg-[var(--accent-bg)] text-[var(--accent)] text-[11px] font-semibold">
                          Default
                        </span>
                      )}
                    </span>
                    <span className="block text-xs text-[var(--ink-2)]">{description}</span>
                  </span>
                </span>
              </button>
            );
          })}
        </div>
        {/* Only a CLI UI with the theme editor reports a custom theme, so the link shows once
            there is one. */}
        {customTheme && (
          <button
            type="button"
            onClick={() => navigate('/appearance')}
            className="self-start flex items-center gap-1.5 text-[13px] font-semibold text-[var(--accent)] hover:underline"
          >
            Edit in the dashboard
            <ArrowRight size={14} />
          </button>
        )}
      </section>

      <section className="flex items-center gap-4 pt-6 border-t border-[var(--line)]">
        <div className="flex-1">
          <h2 className="text-[15px] font-bold text-[var(--ink)]">Mode</h2>
          <p className="mt-0.5 text-[13px] text-[var(--ink-2)]">
            System follows your {isMacOS() ? 'macOS' : 'OS'} setting.
          </p>
        </div>
        <div
          role="radiogroup"
          aria-label="Mode"
          className="flex gap-1 p-1 rounded-[var(--r-btn)] bg-[var(--sunken)]"
        >
          {MODES.map(({ id, label, icon: Icon }) => {
            const checked = modePreference === id;
            return (
              <button
                key={id}
                type="button"
                role="radio"
                aria-checked={checked}
                onClick={() => handleModeChange(id)}
                className={`h-9 px-4 flex items-center gap-1.5 rounded-[var(--r-btn)] text-[13px] border transition-colors ${
                  checked
                    ? 'bg-[var(--surface)] border-[var(--line-2)] font-bold text-[var(--ink)]'
                    : 'border-transparent font-semibold text-[var(--ink-2)] hover:text-[var(--ink)]'
                }`}
              >
                <Icon size={15} />
                {label}
              </button>
            );
          })}
        </div>
      </section>
    </div>
  );
}
