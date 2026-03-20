import { useSearchParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Settings, Palette, FolderOpen, Terminal, Bell, Info } from 'lucide-react';
import GeneralSettings from '../components/settings/GeneralSettings';
import AppearanceSettings from '../components/settings/AppearanceSettings';
import ProjectSettings from '../components/settings/ProjectSettings';
import CliSettings from '../components/settings/CliSettings';
import NotificationSettings from '../components/settings/NotificationSettings';
import AboutSettings from '../components/settings/AboutSettings';

const TABS = [
  { id: 'general', label: 'General', icon: Settings },
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'projects', label: 'Projects', icon: FolderOpen },
  { id: 'cli', label: 'CLI', icon: Terminal },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'about', label: 'About', icon: Info },
] as const;

export default function SettingsPage() {
  const [params, setParams] = useSearchParams();
  const navigate = useNavigate();
  const activeTab = params.get('tab') || 'general';

  const renderContent = () => {
    switch (activeTab) {
      case 'general': return <GeneralSettings />;
      case 'appearance': return <AppearanceSettings />;
      case 'projects': return <ProjectSettings />;
      case 'cli': return <CliSettings />;
      case 'notifications': return <NotificationSettings />;
      case 'about': return <AboutSettings />;
      default: return <GeneralSettings />;
    }
  };

  return (
    <div className="flex h-screen bg-paper">
      {/* Sidebar */}
      <aside
        className="w-60 shrink-0 border-r border-muted flex flex-col"
        style={{ paddingTop: '48px' }}
      >
        <button
          type="button"
          onClick={() => navigate('/')}
          className="flex items-center gap-2 px-4 py-3 text-sm text-pencil-light hover:text-pencil transition-colors"
        >
          <ArrowLeft size={16} strokeWidth={2.5} />
          <span>Back to App</span>
        </button>

        <nav className="flex-1 px-2 py-2 space-y-0.5">
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => setParams({ tab: id })}
              className={`w-full flex items-center gap-2.5 px-3 py-2 text-sm rounded-[var(--radius-sm)] transition-colors ${
                activeTab === id
                  ? 'bg-muted/50 text-pencil font-medium'
                  : 'text-pencil-light hover:text-pencil hover:bg-muted/30'
              }`}
            >
              <Icon size={16} strokeWidth={2.5} />
              <span>{label}</span>
            </button>
          ))}
        </nav>
      </aside>

      {/* Content */}
      <main className="flex-1 overflow-y-auto p-8">
        <div className="max-w-xl mx-auto">
          {renderContent()}
        </div>
      </main>
    </div>
  );
}
