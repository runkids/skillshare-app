import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const version = JSON.parse(read('package.json')).version;
const tag = process.argv[2];

assert.match(version, /^\d+\.\d+\.\d+$/, 'Expected a stable MAJOR.MINOR.PATCH version');
if (tag) assert.equal(tag, `v${version}`, 'Release tag must match package.json');
assert.equal(
  JSON.parse(read('src-tauri/tauri.conf.json')).version,
  version,
  'Tauri version mismatch'
);

const cargoPackage = read('src-tauri/Cargo.toml').split('[package]')[1]?.split(/^\[/m)[0];
assert.equal(
  cargoPackage?.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
  version,
  'Cargo package version mismatch'
);
const lockedPackage = read('src-tauri/Cargo.lock')
  .split('[[package]]')
  .find((entry) => /^name = "skillshare-app"$/m.test(entry));
assert.equal(
  lockedPackage?.match(/^version = "([^"]+)"/m)?.[1],
  version,
  'Cargo.lock version mismatch'
);

console.log(`Release versions match: v${version}`);
