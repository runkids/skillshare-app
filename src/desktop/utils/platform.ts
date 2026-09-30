export type Platform = 'macos' | 'windows' | 'linux';

type NavigatorLike = Pick<Navigator, 'userAgent'> & {
  userAgentData?: { platform?: string };
};

// Frontend-only detection so the title bar can match the native window chrome
// (macOS overlay traffic lights vs. Windows/Linux decorations) without a Tauri plugin.
export function detectPlatform(nav: NavigatorLike = navigator): Platform {
  const hint = nav.userAgentData?.platform || nav.userAgent;
  if (/mac/i.test(hint)) return 'macos';
  if (/\bwin/i.test(hint)) return 'windows';
  return 'linux';
}

export function isMacOS(nav?: NavigatorLike): boolean {
  return detectPlatform(nav) === 'macos';
}
