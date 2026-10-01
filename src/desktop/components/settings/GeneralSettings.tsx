import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import Button from '../../../components/Button';
import Card from '../../../components/Card';
import Input from '../../../components/Input';
import Switch from '../../../components/Switch';
import { tauriBridge } from '../../api/tauri-bridge';
import { useTauri } from '../../context/TauriContext';
import { useProjects } from '../../context/ProjectContext';

export default function GeneralSettings() {
  const navigate = useNavigate();
  const { refresh: refreshAppInfo } = useTauri();
  const { refresh: refreshProjects } = useProjects();
  const [port, setPort] = useState('19420');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [notifyUpdate, setNotifyUpdate] = useState(true);
  const [notifySync, setNotifySync] = useState(true);

  useEffect(() => {
    tauriBridge.getPreferredPort().then((p) => setPort(String(p)));
    tauriBridge
      .getNotifyUpdate()
      .then(setNotifyUpdate)
      .catch(() => {});
    tauriBridge
      .getNotifySync()
      .then(setNotifySync)
      .catch(() => {});
  }, []);

  const handleNotifyUpdate = async (enabled: boolean) => {
    setNotifyUpdate(enabled);
    try {
      await tauriBridge.setNotifyUpdate(enabled);
    } catch (err) {
      setNotifyUpdate(!enabled);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleNotifySync = async (enabled: boolean) => {
    setNotifySync(enabled);
    try {
      await tauriBridge.setNotifySync(enabled);
    } catch (err) {
      setNotifySync(!enabled);
      setError(err instanceof Error ? err.message : String(err));
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
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        General
      </h1>

      <Card>
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-pencil">Server Port</p>
            <p className="text-xs text-pencil-light mt-0.5">
              CLI server will start on this port (restart required)
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={1024}
              max={65535}
              value={port}
              onChange={(e) => {
                setPort(e.target.value);
                setSaved(false);
              }}
              className="!w-32 !py-1.5 text-sm"
            />
            <Button size="sm" onClick={handleSave}>
              {saved ? 'Saved' : 'Save'}
            </Button>
          </div>
        </div>
        {error && <p className="text-danger text-xs mt-2">{error}</p>}
      </Card>

      <Card>
        <div className="flex items-center justify-between gap-4">
          <div>
            <p id="notify-update" className="text-sm font-medium text-pencil">
              Update notifications
            </p>
            <p id="notify-update-hint" className="text-xs text-pencil-light mt-0.5">
              Notify me about new app and CLI versions while the app is in the background
            </p>
          </div>
          <Switch
            checked={notifyUpdate}
            onChange={handleNotifyUpdate}
            labelledBy="notify-update"
            describedBy="notify-update-hint"
          />
        </div>
      </Card>

      <Card>
        <div className="flex items-center justify-between gap-4">
          <div>
            <p id="notify-sync" className="text-sm font-medium text-pencil">
              Sync notifications
            </p>
            <p id="notify-sync-hint" className="text-xs text-pencil-light mt-0.5">
              Show the result when Quick Sync runs from the menu bar
            </p>
          </div>
          <Switch
            checked={notifySync}
            onChange={handleNotifySync}
            labelledBy="notify-sync"
            describedBy="notify-sync-hint"
          />
        </div>
      </Card>

      {/* Danger zone */}
      <div className="pt-4">
        <h2 className="text-sm font-semibold text-danger uppercase tracking-wider mb-3">
          Danger Zone
        </h2>
        <Card className="border-danger/30">
          <div className="space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium text-pencil">Reset app data</p>
                <p className="text-xs text-pencil-light mt-0.5">
                  Return the app to first launch and run setup again.
                </p>
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={confirmReset ? handleReset : () => setConfirmReset(true)}
                loading={resetting}
                className="text-danger hover:text-danger shrink-0"
              >
                {confirmReset ? 'Confirm' : 'Reset'}
              </Button>
            </div>
            <div className="grid grid-cols-2 gap-3 text-xs">
              <div className="rounded-[var(--r-ctl)] bg-[var(--bad-bg)] px-3 py-2.5">
                <p className="font-semibold text-[var(--bad)]">Cleared</p>
                <ul className="mt-1 space-y-0.5 text-[var(--ink-2)] list-disc pl-4">
                  <li>The app’s project list</li>
                  <li>Server port and notification settings</li>
                  <li>Remembered CLI location and version</li>
                </ul>
              </div>
              <div className="rounded-[var(--r-ctl)] bg-[var(--sunken)] px-3 py-2.5">
                <p className="font-semibold text-[var(--ink)]">Kept</p>
                <ul className="mt-1 space-y-0.5 text-[var(--ink-2)] list-disc pl-4">
                  <li>Your skills and the CLI’s own config</li>
                  <li>Project folders on disk</li>
                  <li>The skillshare CLI, including a copy the app downloaded</li>
                </ul>
              </div>
            </div>
            {confirmReset && (
              <p className="text-xs text-danger">
                The local server stops and setup starts over. Nothing on the Kept list is touched.{' '}
                <button
                  type="button"
                  onClick={() => setConfirmReset(false)}
                  className="underline hover:no-underline"
                >
                  Cancel
                </button>
              </p>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
