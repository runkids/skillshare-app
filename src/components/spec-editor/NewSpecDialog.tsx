/**
 * NewSpecDialog
 * Modal dialog for creating a new spec: pick a schema and enter a title.
 */

import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogClose,
  DialogFooter,
} from '../ui/Dialog';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { useSchemas } from '../../hooks/useSchemas';
import type { SelectOption } from '../ui/Select';

interface NewSpecDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectDir: string;
  onCreateSpec: (schemaName: string, title: string) => Promise<void>;
}

export function NewSpecDialog({
  open,
  onOpenChange,
  projectDir,
  onCreateSpec,
}: NewSpecDialogProps) {
  const { schemas, loading: schemasLoading } = useSchemas(projectDir);
  const [selectedSchema, setSelectedSchema] = useState('');
  const [title, setTitle] = useState('');
  const [creating, setCreating] = useState(false);

  const schemaOptions: SelectOption[] = schemas.map((s) => ({
    value: s.name,
    label: s.display_name || s.name,
    description: s.description,
  }));

  const handleCreate = async () => {
    if (!selectedSchema || !title.trim()) return;
    try {
      setCreating(true);
      await onCreateSpec(selectedSchema, title.trim());
      // Reset and close
      setSelectedSchema('');
      setTitle('');
      onOpenChange(false);
    } catch (e) {
      console.error('Failed to create spec:', e);
    } finally {
      setCreating(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && selectedSchema && title.trim() && !creating) {
      e.preventDefault();
      handleCreate();
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogClose onClick={() => onOpenChange(false)} />
        <DialogHeader>
          <DialogTitle>New Spec</DialogTitle>
        </DialogHeader>

        <div className="space-y-4" onKeyDown={handleKeyDown}>
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">Schema</label>
            <Select
              value={selectedSchema}
              onValueChange={setSelectedSchema}
              options={schemaOptions}
              placeholder="Select a schema..."
              loading={schemasLoading}
              aria-label="Schema"
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">Title</label>
            <Input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Enter spec title..."
              autoFocus
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={creating}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={!selectedSchema || !title.trim() || creating}>
            {creating ? 'Creating...' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
