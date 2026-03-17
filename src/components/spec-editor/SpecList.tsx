/**
 * SpecList
 * Dashboard view showing all specs with filtering, creation, and selection.
 */

import { useState, useEffect, useMemo, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Plus, FileText, Filter } from 'lucide-react';
import { Button } from '../ui/Button';
import { Select, type SelectOption } from '../ui/Select';
import { EmptyState } from '../ui/EmptyState';
import { useSpecs } from '../../hooks/useSpecs';
import { useSchemas } from '../../hooks/useSchemas';
import { NewSpecDialog } from './NewSpecDialog';
import type { SpecListItem } from '../../types/spec';

// ---------------------------------------------------------------------------
// Status badge configuration
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<string, string> = {
  draft: 'bg-zinc-500/20 text-zinc-400 border-zinc-500/30',
  active: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
  review: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  implement: 'bg-green-500/20 text-green-400 border-green-500/30',
  verify: 'bg-cyan-500/20 text-cyan-400 border-cyan-500/30',
  archived: 'bg-slate-500/20 text-slate-400 border-slate-500/30',
};

function statusBadgeClass(status: string): string {
  return STATUS_COLORS[status] ?? STATUS_COLORS['draft'];
}

// ---------------------------------------------------------------------------
// Relative time helper
// ---------------------------------------------------------------------------

function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  if (diffMs < 0) return 'just now';

  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return 'just now';

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;

  const years = Math.floor(months / 12);
  return `${years}y ago`;
}

// ---------------------------------------------------------------------------
// SpecCard
// ---------------------------------------------------------------------------

function SpecCard({ spec, onClick }: { spec: SpecListItem; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full text-left p-4 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors duration-150 focus:outline-none focus:ring-2 focus:ring-ring"
    >
      {/* Title */}
      <h3 className="text-sm font-medium text-foreground truncate">{spec.title}</h3>

      {/* Badges row */}
      <div className="flex items-center gap-2 mt-2 flex-wrap">
        {/* Schema badge */}
        <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium bg-purple-500/20 text-purple-400 border border-purple-500/30">
          {spec.schemaId}
        </span>

        {/* Status badge */}
        <span
          className={`inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium border ${statusBadgeClass(spec.status)}`}
        >
          {spec.status}
        </span>

        {/* Workflow phase badge (if present) */}
        {spec.workflowPhase && (
          <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-500/20 text-indigo-400 border border-indigo-500/30">
            {spec.workflowPhase}
          </span>
        )}
      </div>

      {/* Updated time */}
      <p className="text-[11px] text-muted-foreground mt-2">
        Updated {relativeTime(spec.updatedAt)}
      </p>
    </button>
  );
}

// ---------------------------------------------------------------------------
// SpecList (main export)
// ---------------------------------------------------------------------------

interface SpecListProps {
  projectDir: string;
  onSelectSpec: (specId: string) => void;
}

const STATUS_FILTER_OPTIONS: SelectOption[] = [
  { value: '', label: 'All statuses' },
  { value: 'draft', label: 'Draft' },
  { value: 'active', label: 'Active' },
  { value: 'review', label: 'Review' },
  { value: 'implement', label: 'Implement' },
  { value: 'verify', label: 'Verify' },
  { value: 'archived', label: 'Archived' },
];

export function SpecList({ projectDir, onSelectSpec }: SpecListProps) {
  const [statusFilter, setStatusFilter] = useState('');
  const [schemaFilter, setSchemaFilter] = useState('');
  const [newDialogOpen, setNewDialogOpen] = useState(false);

  const { specs, loading, error, refresh, createSpec } = useSpecs(projectDir, {
    status: statusFilter || undefined,
  });

  const { schemas } = useSchemas(projectDir);

  // Schema filter options derived from loaded schemas
  const schemaFilterOptions: SelectOption[] = useMemo(
    () => [
      { value: '', label: 'All schemas' },
      ...schemas.map((s) => ({
        value: s.name,
        label: s.display_name || s.name,
      })),
    ],
    [schemas]
  );

  // Apply client-side schema filter (backend only supports status + workflowPhase filters)
  const filteredSpecs = useMemo(() => {
    if (!schemaFilter) return specs;
    return specs.filter((s) => s.schemaId === schemaFilter);
  }, [specs, schemaFilter]);

  // Listen for Tauri events to auto-refresh
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen('specforge://spec-changed', () => {
      refresh();
    }).then((unlisten) => unlisteners.push(unlisten));

    listen('specforge://spec-deleted', () => {
      refresh();
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [refresh]);

  const handleCreateSpec = useCallback(
    async (schemaName: string, title: string) => {
      await createSpec(schemaName, title);
    },
    [createSpec]
  );

  // Loading state
  if (loading && specs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-muted-foreground">Loading specs...</div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-destructive">{error}</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-3 p-4 border-b border-border flex-shrink-0">
        <Filter className="w-4 h-4 text-muted-foreground flex-shrink-0" />

        <div className="w-40">
          <Select
            value={statusFilter}
            onValueChange={setStatusFilter}
            options={STATUS_FILTER_OPTIONS}
            size="sm"
            aria-label="Filter by status"
          />
        </div>

        <div className="w-40">
          <Select
            value={schemaFilter}
            onValueChange={setSchemaFilter}
            options={schemaFilterOptions}
            size="sm"
            aria-label="Filter by schema"
          />
        </div>

        <div className="flex-1" />

        <Button size="sm" onClick={() => setNewDialogOpen(true)}>
          <Plus className="w-4 h-4 mr-1.5" />
          New Spec
        </Button>
      </div>

      {/* Spec grid / empty state */}
      <div className="flex-1 overflow-auto p-4">
        {filteredSpecs.length === 0 ? (
          <EmptyState
            icon={FileText}
            title="No specs yet"
            description="No specs yet. Create your first spec to get started."
            variant="purple"
            action={{
              label: 'New Spec',
              onClick: () => setNewDialogOpen(true),
              icon: Plus,
            }}
          />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
            {filteredSpecs.map((spec) => (
              <SpecCard key={spec.id} spec={spec} onClick={() => onSelectSpec(spec.id)} />
            ))}
          </div>
        )}
      </div>

      {/* New Spec Dialog */}
      <NewSpecDialog
        open={newDialogOpen}
        onOpenChange={setNewDialogOpen}
        projectDir={projectDir}
        onCreateSpec={handleCreateSpec}
      />
    </div>
  );
}
