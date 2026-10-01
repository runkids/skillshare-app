import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Whether the main window is in full screen, where macOS hides the traffic lights. */
export function useFullscreen() {
  const [fullscreen, setFullscreen] = useState(false);
  useEffect(() => {
    if (!isTauri()) return;
    const win = getCurrentWindow();
    const sync = () => {
      win
        .isFullscreen()
        .then(setFullscreen)
        .catch(() => {});
    };
    sync();
    // Entering or leaving full screen resizes the window.
    const off = win.onResized(sync);
    return () => {
      void off.then((f) => f());
    };
  }, []);
  return fullscreen;
}
