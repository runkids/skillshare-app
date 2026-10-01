import { useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  QUICK_ACTIONS_OPENED_EVENT,
  tauriBridge,
  type QuickActionsContext,
  type SkillSearchResult,
} from '../../api/tauri-bridge';

/** A pasted install source (URL, git address or owner/repo) rather than search words. */
export function sourceFromQuery(query: string): string | null {
  const q = query.trim();
  if (!q || /\s/.test(q) || q.startsWith('-')) return null;
  if (/^(https?:\/\/|git@)/.test(q)) return q;
  if (/^[\w.-]+\/[\w.-]+(\/[\w./-]*)?$/.test(q)) return q;
  return null;
}

/** The single result offered for a pasted source; an empty skill installs all it holds. */
function sourceResult(source: string): SkillSearchResult {
  return { name: source, description: 'Install from this source', source, skill: '' };
}

export function useQuickActions() {
  const [context, setContext] = useState<QuickActionsContext | null>(null);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SkillSearchResult[]>([]);
  const [selected, setSelected] = useState(0);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [install, setInstall] = useState<SkillSearchResult | null>(null);
  const [progress, setProgress] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const running = useRef(false);

  useEffect(() => {
    input.current?.focus();
  }, [context, install]);

  useEffect(() => {
    let active = true;
    const refresh = () => {
      input.current?.focus();
      if (running.current) return;
      tauriBridge.getQuickActionsContext().then(
        (value) => {
          if (active) {
            setContext(value);
            setError(null);
          }
        },
        (err) => {
          if (active) {
            setContext(null);
            setError(String(err));
          }
        }
      );
    };
    refresh();
    const unlisten = isTauri() ? listen(QUICK_ACTIONS_OPENED_EVENT, refresh) : null;
    return () => {
      active = false;
      void unlisten?.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (!query.trim() || !context || install || sourceFromQuery(query)) return;
    let active = true;
    const timer = setTimeout(() => {
      setSearching(true);
      tauriBridge.quickSearch(query.trim(), context.projectId).then(
        (value) => {
          if (active) {
            setResults(value);
            setSearching(false);
          }
        },
        (err) => {
          if (active) {
            setError(String(err));
            setSearching(false);
          }
        }
      );
    }, 350);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [query, context, install]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        void tauriBridge.closeQuickActions().catch((err) => setError(String(err)));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const runAction = async () => {
    if (!context || running.current) return;
    setError(null);
    setOutput(null);
    running.current = true;
    setProgress('Installing skill…');
    try {
      if (install) {
        const result = await tauriBridge.quickInstall(
          install.source,
          install.skill,
          context.projectId
        );
        setOutput(result || `Installed ${install.name}`);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      running.current = false;
      setProgress(null);
    }
  };

  const back = () => {
    setInstall(null);
    setOutput(null);
    setError(null);
  };

  const changeQuery = (value: string) => {
    const source = sourceFromQuery(value);
    setQuery(value);
    setResults(source ? [sourceResult(source)] : []);
    setSelected(0);
    setSearching(!source && !!value.trim());
    setError(null);
  };

  const close = () => {
    void tauriBridge.closeQuickActions().catch((err) => setError(String(err)));
  };

  const reviewSelected = () => {
    if (!searching && results[selected]) setInstall(results[selected]);
  };

  const moveSelection = (delta: number) => {
    setSelected((current) => Math.max(0, Math.min(results.length - 1, current + delta)));
  };

  return {
    context,
    query,
    results,
    selected,
    searching,
    error,
    install,
    progress,
    output,
    input,
    setInstall,
    setSelected,
    runAction,
    back,
    changeQuery,
    close,
    reviewSelected,
    moveSelection,
  };
}
