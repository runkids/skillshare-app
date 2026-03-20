// src/desktop/hooks/useSkillshareStatus.ts
import { useState, useEffect, useCallback, useRef } from 'react';
import { tauriBridge } from '../api/tauri-bridge';

export interface SkillshareStatus {
  syncStatus: 'synced' | 'pending' | 'error' | 'unknown';
  pendingCount?: number;
  skillsCount: number;
  targetsCount: number;
  lastSyncTime: string | null;
}

interface UseSkillshareStatusResult {
  status: SkillshareStatus | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/** Parse `skillshare status` text output into structured data. */
export function parseSkillshareStatus(output: string): SkillshareStatus {
  const lines = output.split('\n');
  let syncStatus: SkillshareStatus['syncStatus'] = 'unknown';
  let pendingCount: number | undefined;
  let skillsCount = 0;
  let targetsCount = 0;
  let lastSyncTime: string | null = null;

  for (const line of lines) {
    const trimmed = line.trim();

    if (trimmed.toLowerCase().startsWith('status:')) {
      const value = trimmed.slice('status:'.length).trim().toLowerCase();
      if (value.includes('synced') || value.includes('clean')) {
        syncStatus = 'synced';
      } else if (value.includes('pending') || value.includes('dirty') || value.includes('modified')) {
        syncStatus = 'pending';
        const match = value.match(/(\d+)/);
        if (match) pendingCount = parseInt(match[1], 10);
      } else if (value.includes('error')) {
        syncStatus = 'error';
      }
    }

    if (trimmed.toLowerCase().startsWith('skills:')) {
      const match = trimmed.match(/(\d+)/);
      if (match) skillsCount = parseInt(match[1], 10);
    }

    if (trimmed.toLowerCase().startsWith('targets:')) {
      const match = trimmed.match(/(\d+)/);
      if (match) targetsCount = parseInt(match[1], 10);
    }

    if (trimmed.toLowerCase().startsWith('last sync:')) {
      lastSyncTime = trimmed.slice('last sync:'.length).trim();
    }
  }

  return { syncStatus, pendingCount, skillsCount, targetsCount, lastSyncTime };
}

export function useSkillshareStatus(projectPath: string | null): UseSkillshareStatusResult {
  const [status, setStatus] = useState<SkillshareStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refreshCounter = useRef(0);

  const fetchStatus = useCallback(async () => {
    if (!projectPath || projectPath === '~') {
      setStatus(null);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const cliPath = await tauriBridge.detectCli();
      if (!cliPath) {
        setError('CLI not found');
        setStatus(null);
        return;
      }

      const output = await tauriBridge.runCli(cliPath, ['status'], projectPath);
      const parsed = parseSkillshareStatus(output);
      setStatus(parsed);
    } catch (e) {
      setError('Status unavailable');
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }, [projectPath]);

  // Fetch on mount and when projectPath changes
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const refresh = useCallback(() => {
    refreshCounter.current += 1;
    fetchStatus();
  }, [fetchStatus]);

  return { status, loading, error, refresh };
}
