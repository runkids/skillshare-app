import { useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  QUICK_ACTIONS_OPENED_EVENT,
  tauriBridge,
  type QuickActionsContext,
  type SkillSearchResult,
} from '../../api/tauri-bridge';

export function useQuickActions() {
  const [context, setContext] = useState<QuickActionsContext | null>(null);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SkillSearchResult[]>([]);
  const [selected, setSelected] = useState(0);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [install, setInstall] = useState<SkillSearchResult | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const [progress, setProgress] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const running = useRef(false);

  useEffect(() => {
    input.current?.focus();
  }, [context, install, creating]);

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
    if (!query.trim() || !context || install || creating) return;
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
  }, [query, context, install, creating]);

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
    setProgress(creating ? 'Creating skill…' : 'Installing skill…');
    try {
      if (creating) {
        const result = await tauriBridge.quickNewSkill(name.trim(), context.projectId);
        setOutput(
          result.openError
            ? `Created ${result.path}. Could not open the editor: ${result.openError}`
            : `Created and opened ${result.path}`
        );
      } else if (install) {
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
    setCreating(false);
    setOutput(null);
    setError(null);
  };

  const changeQuery = (value: string) => {
    setQuery(value);
    setResults([]);
    setSelected(0);
    setSearching(!!value.trim());
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
    creating,
    name,
    progress,
    output,
    input,
    setName,
    setCreating,
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
