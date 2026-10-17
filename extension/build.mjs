// Builds the extension for Chrome-family browsers and Firefox.
//
//   node build.mjs            -> dist/chrome, dist/firefox and the two .zip files
//
// Plain Node, no dependencies. The only external program is `zip`.
// Running it twice produces byte-identical zip files (file order and
// timestamps are fixed), so a release build can be compared with a local one.

import { spawnSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..');

export const GECKO_ID = 'tsumiba@hjosugi.github.io';
export const FIREFOX_MIN_VERSION = '128.0';
export const ICON_SIZES = [16, 32, 48, 128];
const FIXED_TIME = new Date('2020-01-01T00:00:00Z');

/** The workspace version, read from the root Cargo.toml. */
export function workspaceVersion(cargoToml = join(repoRoot, 'Cargo.toml')) {
  const text = readFileSync(cargoToml, 'utf8');
  const section = text.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/);
  const found = section && section[1].match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!found) throw new Error(`could not find [workspace.package] version in ${cargoToml}`);
  return found[1];
}

/**
 * Chrome wants 1-4 dot-separated integers. A pre-release such as `0.2.0-rc.1`
 * goes in `version_name`, with the numeric part in `version`.
 */
export function manifestVersion(cargoVersion) {
  const m = cargoVersion.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+.*)?$/);
  if (!m) throw new Error(`"${cargoVersion}" is not a semantic version`);
  const version = `${m[1]}.${m[2]}.${m[3]}`;
  return m[4] ? { version, version_name: cargoVersion } : { version };
}

/** The manifest for one browser, from the shared base. */
export function buildManifest(browser, cargoVersion) {
  const base = JSON.parse(readFileSync(join(here, 'manifest.base.json'), 'utf8'));
  const manifest = { ...base, ...manifestVersion(cargoVersion) };
  if (browser === 'chrome') {
    manifest.background = { service_worker: 'background.js', type: 'module' };
  } else if (browser === 'firefox') {
    // Firefox runs MV3 background code as an event page, not a service worker.
    manifest.background = { scripts: ['background.js'], type: 'module' };
    manifest.browser_specific_settings = {
      gecko: {
        id: GECKO_ID,
        strict_min_version: FIREFOX_MIN_VERSION,
        // Nothing leaves the machine except requests to the local app.
        data_collection_permissions: { required: ['none'] },
      },
    };
  } else {
    throw new Error(`unknown browser "${browser}"`);
  }
  return manifest;
}

function listFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...listFiles(full));
    else out.push(full);
  }
  return out;
}

function copyTree(from, to) {
  mkdirSync(to, { recursive: true });
  for (const name of readdirSync(from).sort()) {
    const src = join(from, name);
    const dst = join(to, name);
    if (statSync(src).isDirectory()) copyTree(src, dst);
    else copyFileSync(src, dst);
  }
}

/** Every file name a manifest points at. */
export function referencedFiles(manifest) {
  const files = new Set();
  const add = (f) => f && files.add(f);
  Object.values(manifest.icons ?? {}).forEach(add);
  Object.values(manifest.action?.default_icon ?? {}).forEach(add);
  add(manifest.action?.default_popup);
  add(manifest.options_ui?.page);
  add(manifest.background?.service_worker);
  (manifest.background?.scripts ?? []).forEach(add);
  return [...files];
}

/**
 * Build both browsers. `dist` is wiped first. With `zip: false` only the
 * directories are written (the unit tests use that).
 */
export function build({ dist = join(here, 'dist'), zip = true, cargoToml } = {}) {
  const version = workspaceVersion(cargoToml);
  rmSync(dist, { recursive: true, force: true });
  mkdirSync(dist, { recursive: true });

  const results = [];
  for (const browser of ['chrome', 'firefox']) {
    const out = join(dist, browser);
    copyTree(join(here, 'src'), out);

    mkdirSync(join(out, 'icons'), { recursive: true });
    for (const size of ICON_SIZES) {
      const icon = join(repoRoot, 'assets', 'icons', `tsumiba-${size}.png`);
      if (!existsSync(icon)) throw new Error(`missing icon ${icon}`);
      copyFileSync(icon, join(out, 'icons', `tsumiba-${size}.png`));
    }

    const manifest = buildManifest(browser, version);
    writeFileSync(join(out, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');

    for (const file of referencedFiles(manifest)) {
      if (!existsSync(join(out, file))) {
        throw new Error(`${browser}: the manifest refers to ${file}, which is not in the build`);
      }
    }

    let zipPath = null;
    if (zip) {
      zipPath = join(dist, `tsumiba-extension-${browser}-${version}.zip`);
      makeZip(out, zipPath);
    }
    results.push({ browser, dir: out, zip: zipPath, version });
  }
  return results;
}

function makeZip(dir, zipPath) {
  const files = listFiles(dir).map((f) => relative(dir, f).split('\\').join('/'));
  for (const f of files) utimesSync(join(dir, f), FIXED_TIME, FIXED_TIME);
  // `-X` drops extra file attributes, `-@` reads the (sorted) names from stdin.
  const result = spawnSync('zip', ['-X', '-q', zipPath, '-@'], {
    cwd: dir,
    input: files.join('\n') + '\n',
    encoding: 'utf8',
  });
  if (result.error) {
    throw new Error(`could not run \`zip\`: ${result.error.message}. Install zip and try again.`);
  }
  if (result.status !== 0) {
    throw new Error(`zip failed (${result.status}): ${result.stderr || result.stdout}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    for (const r of build()) {
      console.log(`${r.browser.padEnd(8)} ${relative(process.cwd(), r.dir)}  ${relative(process.cwd(), r.zip)}`);
    }
  } catch (error) {
    console.error(`build failed: ${error.message}`);
    process.exit(1);
  }
}
