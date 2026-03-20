import { useState, useEffect } from 'react';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import Card from '../../../components/Card';
import Button from '../../../components/Button';
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
  const [permissionStatus, setPermissionStatus] = useState<string | null>(null);

  useEffect(() => {
    tauriBridge.getNotifySync().then(setSyncNotify);
    tauriBridge.getNotifyUpdate().then(setUpdateNotify);
    // Check notification permission
    isPermissionGranted().then((granted) => {
      setPermissionStatus(granted ? 'granted' : 'denied');
    });
  }, []);

  const handleSyncChange = async (v: boolean) => {
    setSyncNotify(v);
    await tauriBridge.setNotifySync(v);
  };

  const handleUpdateChange = async (v: boolean) => {
    setUpdateNotify(v);
    await tauriBridge.setNotifyUpdate(v);
  };

  const handleRequestPermission = async () => {
    const permission = await requestPermission();
    setPermissionStatus(permission === 'granted' ? 'granted' : 'denied');
  };

  const handleTest = async () => {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
      setPermissionStatus(granted ? 'granted' : 'denied');
    }
    if (granted) {
      sendNotification({ title: 'Skillshare App', body: 'Notifications are working!' });
    }
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        Notifications
      </h1>

      {permissionStatus === 'denied' && (
        <Card>
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-pencil">Permission Required</p>
              <p className="text-xs text-pencil-light mt-0.5">
                macOS needs permission to show notifications
              </p>
            </div>
            <Button size="sm" onClick={handleRequestPermission}>
              Grant Permission
            </Button>
          </div>
        </Card>
      )}

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

      <Button variant="secondary" size="sm" onClick={handleTest}>
        Send Test Notification
      </Button>
    </div>
  );
}
