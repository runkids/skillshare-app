/**
 * useSchemas hook
 * Loads available schema definitions from the backend.
 */

import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect } from 'react';
import type { SchemaDefinition } from '../types/schema';

export function useSchemas(projectDir: string) {
  const [schemas, setSchemas] = useState<SchemaDefinition[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<SchemaDefinition[]>('list_schemas', { projectDir })
      .then(setSchemas)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [projectDir]);

  return { schemas, loading };
}
