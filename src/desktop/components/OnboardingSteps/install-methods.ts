import type { InstallPlatform } from '../../api/tauri-bridge';

export type InstallMethodId = 'brew' | 'script' | 'powershell' | 'app-copy';

export interface InstallMethod {
  id: InstallMethodId;
  label: string;
  /** Exact command the app runs; null when the app downloads the binary itself. */
  command: string | null;
  note: string;
}

const SH_URL = 'https://raw.githubusercontent.com/runkids/skillshare/main/install.sh';
const PS1_URL = 'https://raw.githubusercontent.com/runkids/skillshare/main/install.ps1';

const BREW: InstallMethod = {
  id: 'brew',
  label: 'Homebrew',
  command: 'brew install skillshare',
  note: 'Installs with Homebrew, so the CLI also works in your terminal. Updates with brew upgrade.',
};

// Installs to ~/.local/bin: the default /usr/local/bin needs sudo, which cannot prompt in the app.
const SCRIPT: InstallMethod = {
  id: 'script',
  label: 'Install script',
  command: `mkdir -p "$HOME/.local/bin" && curl -fsSL ${SH_URL} | INSTALL_DIR="$HOME/.local/bin" sh`,
  note: 'Official install script from the skillshare repository. Installs to ~/.local/bin.',
};

const POWERSHELL: InstallMethod = {
  id: 'powershell',
  label: 'PowerShell script',
  command: `irm ${PS1_URL} | iex`,
  note: 'Official PowerShell script from the skillshare repository. Installs to %LOCALAPPDATA%\\Programs\\skillshare.',
};

const APP_COPY: InstallMethod = {
  id: 'app-copy',
  label: 'App-only copy',
  command: null,
  note: "Downloads the latest GitHub release into the app's own folder. The CLI is not added to your terminal PATH.",
};

export function getInstallMethods(platform: InstallPlatform): {
  methods: InstallMethod[];
  recommended: InstallMethodId;
} {
  if (platform.os === 'windows') {
    return { methods: [POWERSHELL, APP_COPY], recommended: 'powershell' };
  }
  if (platform.os === 'macos' && platform.brew) {
    return { methods: [BREW, SCRIPT, APP_COPY], recommended: 'brew' };
  }
  return { methods: [SCRIPT, APP_COPY], recommended: 'script' };
}

export function platformLabel(platform: InstallPlatform): string {
  const os = { macos: 'macOS', linux: 'Linux', windows: 'Windows' }[platform.os];
  const arch =
    platform.os === 'macos'
      ? platform.arch === 'arm64'
        ? 'Apple silicon'
        : 'Intel'
      : platform.arch === 'arm64'
        ? 'arm64'
        : 'x64';
  return `${os} · ${arch}`;
}
