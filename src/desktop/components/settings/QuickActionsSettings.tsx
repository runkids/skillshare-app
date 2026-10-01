import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import Button from '../../../components/Button';
import Card from '../../../components/Card';
import Input from '../../../components/Input';
import Switch from '../../../components/Switch';
import { tauriBridge } from '../../api/tauri-bridge';
import { isMacOS } from '../../utils/platform';

export default function QuickActionsSettings() {
  const [enabled, setEnabled] = useState(true);
  const [shortcut, setShortcut] = useState(isMacOS() ? 'Command+Shift+K' : 'Control+Shift+K');
  const [ready, setReady] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    tauriBridge.getQuickActionsSettings().then(
      (settings) => {
        if (!active) return;
        setEnabled(settings.enabled);
        setShortcut(settings.shortcut);
        setError(settings.error);
        setReady(true);
      },
      (err) => {
        if (active) setError(String(err));
      }
    );
    return () => {
      active = false;
    };
  }, []);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await tauriBridge.setQuickActionsSettings(enabled, shortcut.trim());
      setSaved(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <div className="flex items-center justify-between gap-4">
        <div>
          <p id="quick-actions-enabled" className="text-sm font-medium text-pencil">
            Quick Actions shortcut
          </p>
          <p id="quick-actions-hint" className="text-xs text-pencil-light mt-0.5">
            Search, install or create a skill from any app. Quick Actions stays available in the
            tray.
          </p>
        </div>
        <Switch
          checked={enabled}
          onChange={(value) => {
            setEnabled(value);
            setSaved(false);
          }}
          disabled={!ready || saving}
          labelledBy="quick-actions-enabled"
          describedBy="quick-actions-hint"
        />
      </div>
      <div className="flex items-end gap-2 mt-3">
        <div className="flex-1">
          <Input
            label="Keyboard shortcut"
            value={shortcut}
            disabled={!ready || saving}
            onChange={(event) => {
              setShortcut(event.target.value);
              setSaved(false);
            }}
          />
        </div>
        <Button size="sm" disabled={!ready || !shortcut.trim()} loading={saving} onClick={save}>
          {saved ? 'Saved' : 'Save shortcut'}
        </Button>
      </div>
      <p className="text-xs text-pencil-light mt-2">
        Use modifiers joined with +, for example {isMacOS() ? 'Command+Shift+K' : 'Control+Shift+K'}
        .
      </p>
      {error && (
        <p role="alert" className="text-danger text-xs mt-2">
          {error}
        </p>
      )}
    </Card>
  );
}
