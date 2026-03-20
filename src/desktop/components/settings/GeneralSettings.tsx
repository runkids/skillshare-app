import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import Button from '../../../components/Button';
import Card from '../../../components/Card';
import { tauriBridge } from '../../api/tauri-bridge';

export default function GeneralSettings() {
  const navigate = useNavigate();
  const [port, setPort] = useState('19420');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const [resetting, setResetting] = useState(false);

  useEffect(() => {
    tauriBridge.getPreferredPort().then((p) => setPort(String(p)));
  }, []);

  const handleSave = async () => {
    const num = parseInt(port, 10);
    if (isNaN(num) || num < 1024 || num > 65535) {
      setError('Port must be between 1024 and 65535');
      return;
    }
    try {
      setError(null);
      await tauriBridge.setPreferredPort(num);
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
      // Navigate to onboarding since all data is cleared
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
            <input
              type="number"
              min={1024}
              max={65535}
              value={port}
              onChange={(e) => { setPort(e.target.value); setSaved(false); }}
              className="w-24 px-2 py-1 text-sm border border-muted rounded-[var(--radius-sm)] bg-paper text-pencil focus:outline-none focus:border-pencil"
            />
            <Button size="sm" onClick={handleSave}>
              {saved ? 'Saved' : 'Save'}
            </Button>
          </div>
        </div>
        {error && <p className="text-danger text-xs mt-2">{error}</p>}
      </Card>

      {/* Danger zone */}
      <div className="pt-4">
        <h2 className="text-sm font-semibold text-danger uppercase tracking-wider mb-3">
          Danger Zone
        </h2>
        <Card className="border-danger/30">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-pencil">Reset All Data</p>
              <p className="text-xs text-pencil-light mt-0.5">
                Clear all settings, projects, and CLI configuration. App will restart onboarding.
              </p>
            </div>
            {!confirmReset ? (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setConfirmReset(true)}
                className="text-danger hover:text-danger"
              >
                Reset
              </Button>
            ) : (
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setConfirmReset(false)}
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  onClick={handleReset}
                  loading={resetting}
                  className="bg-danger text-paper hover:bg-danger/90"
                >
                  Confirm Reset
                </Button>
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
