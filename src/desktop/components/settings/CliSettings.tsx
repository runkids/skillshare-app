import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Button from '../../../components/Button';
import { useTauri } from '../../context/TauriContext';
import { tauriBridge } from '../../api/tauri-bridge';
import TerminalAccess from '../TerminalAccess';
import { useUpdates } from '../../hooks/useUpdates';
import { SettingsHeader, SettingsList, SettingsRow, StatusDot } from './SettingsLayout';

export default function CliSettings() {
  const { appInfo } = useTauri();
  const { cli: cliUpdate } = useUpdates();
  const navigate = useNavigate();
  const [cliPath, setCliPath] = useState<string | null>(null);

  useEffect(() => {
    tauriBridge
      .detectCli()
      .then(setCliPath)
      .catch(() => {});
  }, []);

  return (
    <div className="flex flex-col gap-7">
      <SettingsHeader title="CLI" subtitle="The skillshare CLI the app runs on." />

      <SettingsList>
        <SettingsRow
          label="Version"
          hint={<span className="font-mono">{appInfo?.cliVersion || 'Unknown'}</span>}
        >
          {cliUpdate ? (
            <Button size="sm" variant="secondary" onClick={() => navigate('/settings?tab=updates')}>
              {cliUpdate} available
            </Button>
          ) : (
            <StatusDot tone="ok">Latest</StatusDot>
          )}
        </SettingsRow>
        <SettingsRow
          label="Installed with"
          hint="How the CLI was installed, which decides how it updates."
        >
          <span className="font-mono text-[11px] font-medium px-1.5 py-0.5 rounded-[var(--r-tag)] bg-[var(--sunken)] text-[var(--ink-2)]">
            {appInfo?.cliSource || 'unknown'}
          </span>
        </SettingsRow>
        {cliPath && (
          <SettingsRow label="Location" hint={<span className="font-mono">{cliPath}</span>} />
        )}
        {cliPath && (
          <div className="flex flex-col gap-3 px-4 py-3">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-semibold text-[var(--ink)]">Terminal</span>
              <span className="text-[13px] text-[var(--ink-2)]">
                Run skillshare outside the app.
              </span>
            </div>
            <TerminalAccess
              cliPath={cliPath}
              fallback={
                <p className="m-0 text-[13px] text-[var(--ink-2)]">
                  skillshare is on your PATH and ready to use.
                </p>
              }
            />
          </div>
        )}
      </SettingsList>

      <p className="m-0 -mt-4 text-[13px] text-[var(--ink-2)]">
        New CLI versions are installed from the{' '}
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
  );
}
