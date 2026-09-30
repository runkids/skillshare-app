import { describe, expect, it } from 'vitest';
import type { InstallPlatform } from '../../api/tauri-bridge';
import { getInstallMethods } from './install-methods';

const ids = (p: InstallPlatform) => getInstallMethods(p).methods.map((m) => m.id);

describe('getInstallMethods', () => {
  it('recommends Homebrew on macOS when brew is on PATH', () => {
    const plan = getInstallMethods({ os: 'macos', arch: 'arm64', brew: true });
    expect(plan.recommended).toBe('brew');
    expect(plan.methods.find((m) => m.id === 'brew')?.command).toBe('brew install skillshare');
  });

  it('recommends the install script on macOS without brew', () => {
    const plan = getInstallMethods({ os: 'macos', arch: 'x64', brew: false });
    expect(plan.recommended).toBe('script');
    expect(ids({ os: 'macos', arch: 'x64', brew: false })).not.toContain('brew');
  });

  it('recommends the install script on Linux even if brew exists', () => {
    const plan = getInstallMethods({ os: 'linux', arch: 'x64', brew: true });
    expect(plan.recommended).toBe('script');
    expect(ids({ os: 'linux', arch: 'x64', brew: true })).toEqual(['script', 'app-copy']);
  });

  it('recommends the PowerShell script on Windows', () => {
    const plan = getInstallMethods({ os: 'windows', arch: 'x64', brew: false });
    expect(plan.recommended).toBe('powershell');
    expect(plan.methods[0].command).toBe(
      'irm https://raw.githubusercontent.com/runkids/skillshare/main/install.ps1 | iex'
    );
  });

  it('always offers the app-only copy last', () => {
    for (const os of ['macos', 'linux', 'windows'] as const) {
      const methods = getInstallMethods({ os, arch: 'arm64', brew: true }).methods;
      expect(methods[methods.length - 1].id).toBe('app-copy');
    }
  });
});
