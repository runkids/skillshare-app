import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Button from '../../../components/Button';
import Input from '../../../components/Input';
import Switch from '../../../components/Switch';
import { tauriBridge, AUTO_SYNC_CHANGED_EVENT } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';
import { SettingsHeader, SettingsList, SettingsRow, SettingsSection } from './SettingsLayout';

export default function GeneralSettings() {
  const navigate = useNavigate();
  const { refresh: refreshAppInfo } = useTauri();
  const { refresh: refreshProjects } = useProjects();
  const [port, setPort] = useState('19420');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toggleError, setToggleError] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [notifySync, setNotifySync] = useState(true);
  const [autoSync, setAutoSync] = useState(false);

  useEffect(() => {
    tauriBridge.getPreferredPort().then((p) => setPort(String(p)));
    tauriBridge
      .getNotifySync()
      .then(setNotifySync)
      .catch(() => {});
    tauriBridge
      .getAutoSync()
      .then(setAutoSync)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<boolean>(AUTO_SYNC_CHANGED_EVENT, (event) =>
      setAutoSync(event.payload)
    );
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  const handleNotifySync = async (enabled: boolean) => {
    setNotifySync(enabled);
    try {
      await tauriBridge.setNotifySync(enabled);
    } catch (err) {
      setNotifySync(!enabled);
      setToggleError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleAutoSync = async (enabled: boolean) => {
    setAutoSync(enabled);
    try {
      await tauriBridge.setAutoSync(enabled);
    } catch (err) {
      setAutoSync(!enabled);
      setToggleError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSave = async () => {
    const num = parseInt(port, 10);
    if (isNaN(num) || num < 1024 || num > 65535) {
      setError('Port must be between 1024 and 65535');
      return;
    }
    try {
      setError(null);
      await tauriBridge.setPreferredPort(num);
      // Restart server with new port
      await tauriBridge.stopServer().catch(() => {});
      const cliPath = await tauriBridge.detectCli();
      if (cliPath) {
        const active = await tauriBridge.getActiveProject();
        await tauriBridge.startServer(cliPath, active?.path);
      }
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleReset = async () => {
    setResetting(true);
    try {
      await tauriBridge.resetAllData();
      // Refresh cached React state so contexts reflect the cleared data
      await Promise.all([refreshAppInfo(), refreshProjects()]);
      navigate('/onboarding');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setResetting(false);
      setConfirmReset(false);
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <SettingsHeader title="General" />

      <div className="flex flex-col gap-2">
        <SettingsList>
          <div>
            <SettingsRow
              id="server-port"
              label="Server port"
              hint="The dashboard runs on this local port. Saving restarts the server."
            >
              <Input
                type="number"
                min={1024}
                max={65535}
                value={port}
                aria-labelledby="server-port"
                onChange={(e) => {
                  setPort(e.target.value);
                  setSaved(false);
                }}
                className="!w-28 !py-1.5 font-mono text-[13px]"
              />
              <Button size="sm" variant="secondary" onClick={handleSave}>
                {saved ? 'Saved' : 'Save'}
              </Button>
            </SettingsRow>
            {error && (
              <p role="alert" className="m-0 -mt-1 px-4 pb-3 text-[13px] text-[var(--bad)]">
                {error}
              </p>
            )}
          </div>
          <SettingsRow
            id="notify-sync"
            label="Sync notifications"
            hint="Show the result when Quick Sync runs from the menu bar."
          >
            <Switch
              checked={notifySync}
              onChange={handleNotifySync}
              labelledBy="notify-sync"
              describedBy="notify-sync-hint"
            />
          </SettingsRow>
          <SettingsRow
            id="auto-sync"
            label="Auto-sync"
            hint="Sync automatically when files in the active project’s skills source change."
          >
            <Switch
              checked={autoSync}
              onChange={handleAutoSync}
              labelledBy="auto-sync"
              describedBy="auto-sync-hint"
            />
          </SettingsRow>
        </SettingsList>
        {toggleError && (
          <p role="alert" className="m-0 text-[13px] text-[var(--bad)]">
            {toggleError}
          </p>
        )}
        <p className="m-0 text-[13px] text-[var(--ink-2)]">
          Update notifications are on the{' '}
          <button
            type="button"
            onClick={() => navigate('/settings?tab=updates')}
            className="p-0 bg-transparent border-0 text-[var(--accent)] hover:underline cursor-pointer"
          >
            Updates
          </button>{' '}
          page.
        </p>
      </div>

      <SettingsSection title="Reset app data" heading="h2">
        <SettingsList>
          <div className="flex items-start gap-4 px-4 py-4">
            <p className="m-0 flex-1 text-sm text-[var(--ink)]">
              Return the app to first launch and run setup again. The local server stops and setup
              starts over.
            </p>
            {confirmReset ? (
              <span className="flex items-center gap-2 shrink-0">
                <Button size="sm" variant="secondary" onClick={() => setConfirmReset(false)}>
                  Cancel
                </Button>
                <Button size="sm" variant="danger" onClick={handleReset} loading={resetting}>
                  Reset now
                </Button>
              </span>
            ) : (
              <Button size="sm" variant="danger" onClick={() => setConfirmReset(true)}>
                Reset app data…
              </Button>
            )}
          </div>
          <div className="grid grid-cols-2 gap-4 px-4 py-4">
            <div>
              <p className="m-0 text-[13px] font-semibold text-[var(--ink)]">Cleared</p>
              <ul className="mt-1.5 mb-0 pl-[18px] flex flex-col gap-[3px] text-[13px] text-[var(--ink-2)]">
                <li>The app’s project list</li>
                <li>Server port and notification settings</li>
                <li>Remembered CLI location and version</li>
              </ul>
            </div>
            <div>
              <p className="m-0 text-[13px] font-semibold text-[var(--ink)]">Kept</p>
              <ul className="mt-1.5 mb-0 pl-[18px] flex flex-col gap-[3px] text-[13px] text-[var(--ink-2)]">
                <li>Your skills and the CLI’s own config</li>
                <li>Project folders on disk</li>
                <li>The skillshare CLI, including a copy the app downloaded</li>
              </ul>
            </div>
          </div>
        </SettingsList>
      </SettingsSection>
    </div>
  );
}
