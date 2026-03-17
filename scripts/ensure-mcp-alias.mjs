import fs from 'node:fs';
import path from 'node:path';

function ensureAlias({ profile }) {
  const projectRoot = process.cwd();
  const targetDir = path.join(projectRoot, 'target', profile);

  const exeSuffix = process.platform === 'win32' ? '.exe' : '';
  const source = path.join(targetDir, `specforge-mcp${exeSuffix}`);
  const dest = path.join(targetDir, `mcp${exeSuffix}`);

  if (!fs.existsSync(source)) {
    console.warn(`[ensure-mcp-alias] Skip: source not found: ${source}`);
    return;
  }

  const shouldCopy =
    !fs.existsSync(dest) ||
    fs.statSync(source).mtimeMs > fs.statSync(dest).mtimeMs ||
    fs.statSync(source).size !== fs.statSync(dest).size;

  if (!shouldCopy) return;

  fs.copyFileSync(source, dest);
  try {
    fs.chmodSync(dest, 0o755);
  } catch {
    // best-effort on non-posix filesystems
  }

  console.log(`[ensure-mcp-alias] Copied: ${source} -> ${dest}`);
}

const profileArg = process.argv.find((arg) => arg.startsWith('--profile='));
const profile = profileArg?.split('=')[1] ?? 'debug';

if (!['debug', 'release'].includes(profile)) {
  console.error('[ensure-mcp-alias] Invalid --profile; expected debug|release');
  process.exit(2);
}

ensureAlias({ profile });

