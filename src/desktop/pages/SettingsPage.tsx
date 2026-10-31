import { useSearchParams, useNavigate } from 'react-router-dom';
import {
  ArrowLeft,
  ArrowUpCircle,
  Settings,
  Palette,
  FolderOpen,
  Terminal,
  Info,
} from 'lucide-react';
import GeneralSettings from '../components/settings/GeneralSettings';
import AppearanceSettings from '../components/settings/AppearanceSettings';
import ProjectSettings from '../components/settings/ProjectSettings';
import CliSettings from '../components/settings/CliSettings';
import AboutSettings from '../components/settings/AboutSettings';
import UpdatesSettings from '../components/settings/UpdatesSettings';
import { isMacOS } from '../utils/platform';
import { useEffect } from 'react';
import { useAppUpdate } from '../hooks/useAppUpdate';
import { useUpdates } from '../hooks/useUpdates';
import { useTauri } from '../context/TauriContext';

const TABS = [
  { id: 'updates', label: 'Updates', icon: ArrowUpCircle },
  { id: 'general', label: 'General', icon: Settings },
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'projects', label: 'Projects', icon: FolderOpen },
  { id: 'cli', label: 'CLI', icon: Terminal },
  { id: 'about', label: 'About', icon: Info },
] as const;

export default function SettingsPage() {
  const [params, setParams] = useSearchParams();
  const navigate = useNavigate();
  const { available: appUpdateAvailable } = useAppUpdate();
  const updates = useUpdates();
  const pending =
    Number(appUpdateAvailable || Boolean(updates.app)) +
    Number(Boolean(updates.cli)) +
    updates.skills.length +
    updates.agents.length +
    updates.plugins.length +
    updates.repositories.length;
  const badge = (tab: string) => (tab === 'updates' ? pending : 0);
  const { refresh: refreshAppInfo } = useTauri();
  // The CLI can be upgraded from its web UI or a terminal; re-read versions on open.
  useEffect(() => {
    refreshAppInfo();
  }, [refreshAppInfo]);
  const activeTab = params.get('tab') || 'general';

  const renderContent = () => {
    switch (activeTab) {
      case 'general':
        return <GeneralSettings />;
      case 'appearance':
        return <AppearanceSettings />;
      case 'projects':
        return <ProjectSettings />;
      case 'cli':
        return <CliSettings />;
      case 'about':
        return <AboutSettings />;
      case 'updates':
        return <UpdatesSettings />;
      default:
        return <GeneralSettings />;
    }
  };

  return (
    <div className="flex h-screen bg-[var(--bg)]">
      {/* Leave room for the macOS overlay traffic lights; this page has no title bar. */}
      <aside
        className="w-[248px] shrink-0 bg-[var(--side)] border-r border-[var(--line)] flex flex-col gap-[18px] px-3.5 pb-5"
        style={{ paddingTop: isMacOS() ? '48px' : '20px' }}
      >
        <button
          type="button"
          onClick={() => navigate('/')}
          className="flex items-center gap-2 h-9 px-2.5 rounded-[var(--r-ctl)] text-sm text-[var(--ink-2)] hover:text-[var(--ink)] hover:bg-[var(--sel)] transition-colors"
        >
          <ArrowLeft size={18} />
          <span>Back to dashboard</span>
        </button>

        <div
          className="px-2.5 text-[17px] font-bold tracking-[-0.01em] text-[var(--ink)]"
          style={{ fontFamily: 'var(--fh)' }}
        >
          Settings
        </div>

        <nav aria-label="Settings" className="flex-1 flex flex-col gap-0.5">
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => setParams({ tab: id })}
              aria-current={activeTab === id ? 'page' : undefined}
              aria-label={badge(id) > 0 ? `${label}, ${badge(id)} available` : undefined}
              className={`w-full flex items-center gap-2.5 h-9 px-3 text-sm rounded-[var(--r-ctl)] transition-colors ${
                activeTab === id
                  ? 'bg-[var(--sel)] text-[var(--sel-ink)] font-semibold'
                  : 'text-[var(--ink-2)] hover:text-[var(--ink)] hover:bg-[var(--sel)]'
              }`}
            >
              <Icon size={18} />
              <span className="flex-1 text-left">{label}</span>
              {badge(id) > 0 && (
                <span className="min-w-5 h-5 px-1.5 flex items-center justify-center text-[11px] font-bold rounded-full bg-[var(--ink)] text-[var(--surface)]">
                  {badge(id)}
                </span>
              )}
            </button>
          ))}
        </nav>
      </aside>

      <main className="flex-1 overflow-y-auto px-16 py-12">
        <div className="max-w-[720px] mx-auto">{renderContent()}</div>
      </main>
    </div>
  );
}
