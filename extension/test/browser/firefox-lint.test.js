// Static validation of the Firefox build with Mozilla's own linter.
//
// This does NOT run the extension in Firefox (that is not automated here); it
// proves the manifest and files are what Firefox and addons.mozilla.org expect.
// Skipped unless `web-ext` is on PATH, e.g.
//
//   nix shell nixpkgs#web-ext -c node --test test/browser/firefox-lint.test.js

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { build } from '../../build.mjs';

const hasWebExt = spawnSync('web-ext', ['--version'], { encoding: 'utf8' }).status === 0;

test('web-ext lint finds nothing in the Firefox build', { skip: hasWebExt ? false : 'web-ext is not on PATH' }, () => {
  const dist = mkdtempSync(join(tmpdir(), 'tsumiba-ext-lint-'));
  try {
    build({ dist, zip: false });
    const result = spawnSync(
      'web-ext',
      ['lint', '--source-dir', join(dist, 'firefox'), '--self-hosted', '--output', 'json'],
      { encoding: 'utf8' },
    );
    const report = JSON.parse(result.stdout);
    const findings = [...report.errors, ...report.warnings, ...report.notices].map((f) => `${f.code}: ${f.message}`);
    assert.deepEqual(findings, []);
    assert.equal(result.status, 0);
  } finally {
    rmSync(dist, { recursive: true, force: true });
  }
});
