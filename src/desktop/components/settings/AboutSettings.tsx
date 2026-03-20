import Card from '../../../components/Card';
import { useTauri } from '../../context/TauriContext';

export default function AboutSettings() {
  const { appInfo } = useTauri();

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-pencil" style={{ fontFamily: 'var(--font-heading)' }}>
        About
      </h1>

      <Card className="divide-y divide-muted">
        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">App</p>
          <span className="text-sm text-pencil-light">Skillshare App v0.1.0</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">CLI Version</p>
          <span className="text-sm text-pencil-light">{appInfo?.cliVersion || 'Unknown'}</span>
        </div>

        <div className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
          <p className="text-sm font-medium text-pencil">GitHub</p>
          <a
            href="https://github.com/runkids/skillshare"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-pencil-light hover:text-pencil underline"
          >
            runkids/skillshare
          </a>
        </div>
      </Card>
    </div>
  );
}
