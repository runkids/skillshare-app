import { useState } from 'react';
import Card from '../../../components/Card';
import Badge from '../../../components/Badge';
import Button from '../../../components/Button';
import { useTauri } from '../../context/TauriContext';
import { tauriBridge } from '../../api/tauri-bridge';

export default function CliSettings() {
  const { appInfo, refresh } = useTauri();
  const [updating, setUpdating] = useState(false);
  const [updateResult, setUpdateResult] = useState<string | null>(null);

  const handleCheckUpdate = async () => {
    setUpdating(true);
    setUpdateResult(null);
    try {
      const available = await tauriBridge.checkCliUpdate();
      if (available) {
        setUpdateResult(`Update available: ${available}`);
      } else {
        setUpdateResult('Already up to date');
        setTimeout(() => setUpdateResult(null), 3000);
      }
    } catch (err) {
      setUpdateResult(err instanceof Error ? err.message : String(err));
    } finally {
      setUpdating(false);
    }
  };

  const handleUpgrade = async () => {
    setUpdating(true);
    try {
      await tauriBridge.upgradeCli();
      await refresh();
      setUpdateResult('Updated successfully');
      setTimeout(() => setUpdateResult(null), 3000);
    } catch (err) {
      setUpdateResult(err instanceof Error ? err.message : String(err));
    } finally {
      setUpdating(false);
    }
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        CLI
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Version</p>
            <p className="text-xs text-pencil-light mt-0.5">Currently installed CLI version</p>
          </div>
          <span className="text-sm text-pencil">{appInfo?.cliVersion || 'Unknown'}</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Source</p>
            <p className="text-xs text-pencil-light mt-0.5">How the CLI was installed</p>
          </div>
          <Badge size="sm">{appInfo?.cliSource || 'unknown'}</Badge>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <div>
            <p className="text-sm font-medium text-pencil">Updates</p>
            {updateResult && (
              <p className="text-xs text-pencil-light mt-0.5">{updateResult}</p>
            )}
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="secondary" onClick={handleCheckUpdate} loading={updating}>
              Check Update
            </Button>
            {updateResult?.includes('available') && (
              <Button size="sm" onClick={handleUpgrade} loading={updating}>
                Update Now
              </Button>
            )}
          </div>
        </div>
      </Card>
    </div>
  );
}
