import { Folder, Globe } from 'lucide-react';
import Card from '../../components/Card';
import Badge from '../../components/Badge';
import type { Project } from '../api/tauri-bridge';

interface ProjectCardProps {
  project: Project;
  isActive?: boolean;
}

export default function ProjectCard({ project, isActive }: ProjectCardProps) {
  const isGlobal = project.projectType === 'global';

  // Truncate long paths: show last 3 segments
  const truncatedPath = (() => {
    const parts = project.path.split('/').filter(Boolean);
    if (parts.length <= 3) return project.path;
    return '.../' + parts.slice(-3).join('/');
  })();

  return (
    <Card>
      <div className="flex items-start gap-3">
        <div
          className={`w-9 h-9 flex items-center justify-center shrink-0 border border-muted rounded-[var(--radius-sm)] ${
            isGlobal ? 'bg-info-light' : 'bg-success-light'
          }`}
        >
          {isGlobal ? (
            <Globe size={16} strokeWidth={2.5} className="text-blue" />
          ) : (
            <Folder size={16} strokeWidth={2.5} className="text-success" />
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="font-medium text-pencil truncate text-sm">
              {project.name}
            </span>
            <Badge variant={isGlobal ? 'info' : 'success'} size="sm">
              {isGlobal ? 'Global' : 'Project'}
            </Badge>
            {isActive && (
              <Badge variant="accent" size="sm">Active</Badge>
            )}
          </div>
          <p
            className="text-xs text-pencil-light font-mono truncate"
            title={project.path}
          >
            {truncatedPath}
          </p>
        </div>
      </div>
    </Card>
  );
}
