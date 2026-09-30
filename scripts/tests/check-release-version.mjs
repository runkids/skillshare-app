import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const script = fileURLToPath(new URL('../check-release-version.mjs', import.meta.url));

function check({ tag = 'v0.0.6', mismatch } = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'skillshare-release-'));
  mkdirSync(join(dir, 'src-tauri'));
  const versions = {
    'package.json': JSON.stringify({ version: '0.0.6' }),
    'src-tauri/tauri.conf.json': JSON.stringify({ version: '0.0.6' }),
    'src-tauri/Cargo.toml':
      '[package]\nname = "skillshare-app"\nversion = "0.0.6"\n\n[dependencies]\nother = "1.0.0"\n',
    'src-tauri/Cargo.lock':
      'version = 4\n\n[[package]]\nname = "other"\nversion = "1.0.0"\n\n[[package]]\nname = "skillshare-app"\nversion = "0.0.6"\n',
  };
  try {
    for (const [path, content] of Object.entries(versions)) {
      writeFileSync(
        join(dir, path),
        path === mismatch ? content.replace('0.0.6', '0.0.5') : content
      );
    }
    return spawnSync(process.execPath, [script, tag], { cwd: dir, encoding: 'utf8' });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('accepts matching app versions without comparing dependency or lockfile format versions', () => {
  const result = check();
  assert.equal(result.status, 0, result.stderr);
});

for (const path of [
  'package.json',
  'src-tauri/tauri.conf.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
]) {
  test(`rejects a stale version in ${path}`, () => {
    assert.notEqual(check({ mismatch: path }).status, 0);
  });
}

for (const tag of ['v0.0.7', 'main', 'v0.0.6-beta.1']) {
  test(`rejects a mismatched release tag: ${tag}`, () => {
    assert.notEqual(check({ tag }).status, 0);
  });
}
