import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { parse } from 'yaml';

test('generates a cask without an unsupported macOS dependency', () => {
  const workflow = parse(
    readFileSync(new URL('../../.github/workflows/update-homebrew.yml', import.meta.url), 'utf8')
  );
  const step = workflow.jobs['update-homebrew'].steps.find((step) => step.name === 'Update Cask');
  const dir = mkdtempSync(join(tmpdir(), 'skillshare-homebrew-'));
  try {
    mkdirSync(join(dir, 'homebrew-tap/Casks'), { recursive: true });
    const result = spawnSync('bash', ['-e', '-c', step.run], {
      cwd: dir,
      env: {
        ...process.env,
        VERSION: '0.8.1',
        SHA256: 'a'.repeat(64),
        GH_REPO: 'runkids/skillshare-app',
      },
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    const cask = readFileSync(join(dir, 'homebrew-tap/Casks/skillshare-app.rb'), 'utf8');
    assert.doesNotMatch(cask, /depends_on\s+macos:/);
    assert.match(cask, /version "0\.8\.1"/);
    assert.ok(cask.includes(`sha256 "${'a'.repeat(64)}"`));
    assert.ok(
      cask.includes(
        'url "https://github.com/runkids/skillshare-app/releases/download/v#{version}/skillshare_#{version}_aarch64.dmg"'
      )
    );
    assert.match(cask, /app "skillshare\.app"/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
