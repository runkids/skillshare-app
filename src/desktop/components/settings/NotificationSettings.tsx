import { useState, useEffect } from 'react';
import Card from '../../../components/Card';
import { tauriBridge } from '../../api/tauri-bridge';

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
        checked ? 'bg-pencil' : 'bg-muted'
      }`}
    >
      <span
        className={`inline-block h-4 w-4 rounded-full bg-paper transition-transform ${
          checked ? 'translate-x-6' : 'translate-x-1'
        }`}
      />
    </button>
  );
}

export default function NotificationSettings() {
  const [syncNotify, setSyncNotify] = useState(true);
  const [updateNotify, setUpdateNotify] = useState(true);

  useEffect(() => {
    tauriBridge.getNotifySync().then(setSyncNotify);
    tauriBridge.getNotifyUpdate().then(setUpdateNotify);
  }, []);

  const handleSyncChange = async (v: boolean) => {
    setSyncNotify(v);
    await tauriBridge.setNotifySync(v);
  };

  const handleUpdateChange = async (v: boolean) => {
    setUpdateNotify(v);
    await tauriBridge.setNotifyUpdate(v);
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        Notifications
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Sync complete</p>
            <p className="text-xs text-pencil-light mt-0.5">Show notification when sync finishes</p>
          </div>
          <Toggle checked={syncNotify} onChange={handleSyncChange} />
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Update available</p>
            <p className="text-xs text-pencil-light mt-0.5">Notify when a new CLI version exists</p>
          </div>
          <Toggle checked={updateNotify} onChange={handleUpdateChange} />
        </div>
      </Card>
    </div>
  );
}
