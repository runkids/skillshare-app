import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppUpdate } from '../hooks/useAppUpdate';
import { checkUpdatesNow, useUpdates } from '../hooks/useUpdates';

/** Handles "Check for Updates…" from the app and tray menus: open About and check right away. */
export default function UpdateCheckListener() {
  const navigate = useNavigate();
  const { recheck } = useAppUpdate();
  useUpdates();

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen('check-for-updates', () => {
      navigate('/settings?tab=about');
      void recheck();
      void checkUpdatesNow().catch(() => {});
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [navigate, recheck]);

  return null;
}
