import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { after, before, test } from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  FIREFOX_MIN_VERSION,
  GECKO_ID,
  ICON_SIZES,
  build,
  buildManifest,
  manifestVersion,
  referencedFiles,
  workspaceVersion,
} from '../../build.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..', '..');

let dist;
let results;
before(() => {
  dist = mkdtempSync(join(tmpdir(), 'tsumiba-ext-build-'));
  results = build({ dist });
});
after(() => rmSync(dist, { recursive: true, force: true }));

const manifestOf = (browser) => JSON.parse(readFileSync(join(dist, browser, 'manifest.json'), 'utf8'));

/** The version from Cargo.toml, parsed independently of build.mjs. */
function cargoVersion() {
  const text = readFileSync(join(repoRoot, 'Cargo.toml'), 'utf8');
  return /\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/.exec(text)[1];
}

for (const browser of ['chrome', 'firefox']) {
  test(`${browser}: a valid Manifest V3 with the documented permissions and nothing more`, () => {
    const m = manifestOf(browser);
    assert.equal(m.manifest_version, 3);
    assert.equal(m.name, 'tsumiba');
    assert.ok(m.description.length > 10 && m.description.length <= 132, 'store descriptions are at most 132 characters');
    assert.deepEqual([...m.permissions].sort(), ['activeTab', 'contextMenus', 'scripting', 'storage']);
    assert.deepEqual([...m.host_permissions].sort(), ['http://127.0.0.1/*', 'http://localhost/*']);
    assert.deepEqual(m.optional_host_permissions, ['<all_urls>']);
    assert.ok(!m.host_permissions.some((p) => p.includes('<all_urls>') || p === '*://*/*' || p.includes('*://')), 'no broad host permission is required');
    assert.ok(!('content_scripts' in m), 'no content scripts: nothing is injected into pages automatically');
    assert.ok(!('web_accessible_resources' in m), 'nothing is exposed to web pages');
    assert.ok(!('content_security_policy' in m), 'the strict default CSP stays in force');
  });

  test(`${browser}: the version is the workspace version`, () => {
    const m = manifestOf(browser);
    assert.equal(workspaceVersion(), cargoVersion());
    assert.equal(m.version, cargoVersion().replace(/-.*$/, ''));
    assert.match(m.version, /^\d+\.\d+\.\d+$/);
  });

  test(`${browser}: every file the manifest names is in the build`, () => {
    const m = manifestOf(browser);
    const files = referencedFiles(m);
    assert.deepEqual(
      [...files].sort(),
      ['background.js', 'icons/tsumiba-128.png', 'icons/tsumiba-16.png', 'icons/tsumiba-32.png', 'icons/tsumiba-48.png', 'options.html', 'popup.html'],
    );
    for (const file of files) assert.ok(existsSync(join(dist, browser, file)), `${browser}: ${file} is missing`);
  });

  test(`${browser}: icons exist at every size and are PNGs`, () => {
    const m = manifestOf(browser);
    for (const size of ICON_SIZES) {
      const file = join(dist, browser, m.icons[String(size)]);
      assert.ok(existsSync(file), file);
      assert.deepEqual([...readFileSync(file).subarray(0, 8)], [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    }
    assert.ok(m.action.default_icon['16'] && m.action.default_icon['32'], 'the toolbar icon is declared');
  });

  test(`${browser}: the popup, options page and shortcut are declared`, () => {
    const m = manifestOf(browser);
    assert.equal(m.action.default_popup, 'popup.html');
    assert.equal(m.options_ui.page, 'options.html');
    assert.equal(m.options_ui.open_in_tab, true);
    assert.equal(m.commands['send-page'].suggested_key.default, 'Alt+Shift+S');
    assert.ok(m.commands['send-page'].description);
  });

  test(`${browser}: the build contains the library but no tests or build files`, () => {
    for (const f of ['lib/api.js', 'lib/format.js', 'lib/images.js', 'lib/i18n.js', 'lib/settings.js', 'lib/ext.js', 'background.js']) {
      assert.ok(existsSync(join(dist, browser, f)), f);
    }
    for (const f of ['test', 'build.mjs', 'package.json', 'manifest.base.json']) {
      assert.ok(!existsSync(join(dist, browser, f)), `${f} must not ship`);
    }
  });
}

test('chrome runs the background as a module service worker', () => {
  const m = manifestOf('chrome');
  assert.deepEqual(m.background, { service_worker: 'background.js', type: 'module' });
  assert.ok(!('browser_specific_settings' in m));
});

test('firefox runs the background as a module event page and has an add-on id', () => {
  const m = manifestOf('firefox');
  assert.deepEqual(m.background, { scripts: ['background.js'], type: 'module' });
  assert.equal(m.browser_specific_settings.gecko.id, GECKO_ID);
  assert.equal(GECKO_ID, 'tsumiba@hjosugi.github.io');
  assert.equal(m.browser_specific_settings.gecko.strict_min_version, FIREFOX_MIN_VERSION);
  assert.deepEqual(m.browser_specific_settings.gecko.data_collection_permissions, { required: ['none'] });
  assert.ok(parseFloat(FIREFOX_MIN_VERSION) >= 128);
});

test('a pre-release goes in version_name; Chrome gets the numeric part', () => {
  assert.deepEqual(manifestVersion('0.1.0'), { version: '0.1.0' });
  assert.deepEqual(manifestVersion('1.2.3-rc.1'), { version: '1.2.3', version_name: '1.2.3-rc.1' });
  assert.deepEqual(manifestVersion('1.2.3+build5'), { version: '1.2.3' });
  assert.throws(() => manifestVersion('banana'), /semantic version/);
  assert.throws(() => manifestVersion('1.2'), /semantic version/);
  assert.equal(buildManifest('chrome', '2.0.0-beta.2').version_name, '2.0.0-beta.2');
});

test('an unknown browser is an error, not a silent default', () => {
  assert.throws(() => buildManifest('netscape', '0.1.0'), /unknown browser/);
});

test('the build writes both zips, with the manifest at the top level', () => {
  const version = cargoVersion();
  for (const browser of ['chrome', 'firefox']) {
    const zip = join(dist, `tsumiba-extension-${browser}-${version}.zip`);
    assert.ok(existsSync(zip), zip);
    const listing = execFileSync('unzip', ['-Z1', zip], { encoding: 'utf8' }).split('\n').filter(Boolean);
    assert.ok(listing.includes('manifest.json'), 'manifest.json must be at the zip root');
    assert.ok(listing.includes('background.js') && listing.includes('popup.html'));
    assert.ok(listing.every((f) => !f.startsWith('/') && !f.includes('..') && !f.startsWith(`${browser}/`)), 'paths are relative to the extension root');
    assert.deepEqual(listing, [...listing].sort(), 'files are in a fixed order');
  }
  assert.equal(results.length, 2);
});

test('building twice gives byte-identical zips', () => {
  const other = mkdtempSync(join(tmpdir(), 'tsumiba-ext-build2-'));
  try {
    build({ dist: other });
    const version = cargoVersion();
    for (const browser of ['chrome', 'firefox']) {
      const name = `tsumiba-extension-${browser}-${version}.zip`;
      assert.ok(readFileSync(join(dist, name)).equals(readFileSync(join(other, name))), `${name} differs between builds`);
    }
  } finally {
    rmSync(other, { recursive: true, force: true });
  }
});

test('a missing workspace version is a loud error', () => {
  const bad = join(dist, 'Cargo.toml');
  assert.throws(() => workspaceVersion(bad), /ENOENT/);
});
