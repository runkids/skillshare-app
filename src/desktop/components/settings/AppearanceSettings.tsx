import Card from '../../../components/Card';
import SegmentedControl from '../../../components/SegmentedControl';
import { useTheme, type Style, type ModePreference } from '../../../context/useTheme';

const STYLES: { id: Style; label: string }[] = [
  { id: 'clean', label: 'Clean' },
  { id: 'playful', label: 'Playful' },
];

const MODES: { id: ModePreference; label: string }[] = [
  { id: 'light', label: 'Light' },
  { id: 'dark', label: 'Dark' },
  { id: 'system', label: 'System' },
];

export default function AppearanceSettings() {
  const { style, setStyle, modePreference, setModePreference } = useTheme();

  const handleStyleChange = (s: Style) => {
    setStyle(s);
    // Style is persisted in localStorage by ThemeContext;
    // Tauri only stores mode preference — no need to call setPreferredTheme here.
  };

  const handleModeChange = (m: ModePreference) => {
    setModePreference(m);
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        Appearance
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Style</p>
            <p className="text-xs text-pencil-light mt-0.5">Visual style of the interface</p>
          </div>
          <SegmentedControl options={STYLES} value={style} onChange={handleStyleChange} />
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Mode</p>
            <p className="text-xs text-pencil-light mt-0.5">Light or dark appearance</p>
          </div>
          <SegmentedControl options={MODES} value={modePreference} onChange={handleModeChange} />
        </div>
      </Card>
    </div>
  );
}
