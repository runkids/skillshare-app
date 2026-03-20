import { useState, useEffect } from 'react';
import Button from '../../../components/Button';
import Card from '../../../components/Card';
import { tauriBridge } from '../../api/tauri-bridge';

export default function GeneralSettings() {
  const [port, setPort] = useState('19420');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    </div>
  );
}
