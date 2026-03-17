/**
 * useSpecs hook
 * Provides spec CRUD operations backed by Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect, useCallback } from 'react';
import type { Spec, SpecListItem } from '../types/spec';

export function useSpecs(
  projectDir: string,
  filters?: { status?: string; workflowPhase?: string }
) {
  const [specs, setSpecs] = useState<SpecListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<SpecListItem[]>('list_specs', {
        projectDir,
        status: filters?.status ?? null,
        workflowPhase: filters?.workflowPhase ?? null,
      });
      setSpecs(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectDir, filters?.status, filters?.workflowPhase]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const createSpec = useCallback(
    async (schemaName: string, title: string) => {
      const spec = await invoke<Spec>('create_spec', {
        schemaName,
        title,
        projectDir,
      });
      await refresh();
      return spec;
    },
    [projectDir, refresh]
  );

  const deleteSpec = useCallback(
    async (id: string) => {
      await invoke('delete_spec', { id, projectDir });
      await refresh();
    },
    [projectDir, refresh]
  );

  return { specs, loading, error, refresh, createSpec, deleteSpec };
}
