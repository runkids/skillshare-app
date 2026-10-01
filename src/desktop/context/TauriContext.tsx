import { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { ReactNode } from 'react';
import { tauriBridge, type AppInfo } from '../api/tauri-bridge';

interface TauriContextValue {
  appInfo: AppInfo | null;
  loading: boolean;
  refresh: () => Promise<void>;
}

const TauriContext = createContext<TauriContextValue>({
  appInfo: null,
  loading: true,
  refresh: async () => {},
});

// eslint-disable-next-line react-refresh/only-export-components -- hook lives beside its provider
export function useTauri() {
  return useContext(TauriContext);
}

export function TauriProvider({ children }: { children: ReactNode }) {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const info = await tauriBridge.getAppState();
      setAppInfo(info);
    } catch {
      // Tauri not available (running in browser dev mode) — leave null
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <TauriContext.Provider value={{ appInfo, loading, refresh }}>{children}</TauriContext.Provider>
  );
}
