#!/usr/bin/env node
// Check the built documentation site (target/docs-site by default):
//
//   * every internal link points at a file that exists, and at an anchor that
//     exists in it;
//   * every image, stylesheet, script and font resolves;
//   * nothing is *loaded* from the network: no <script src>, <link href>,
//     <img src>, <source>, <iframe> or CSS url() with an http(s):// or
//     protocol-relative address. Ordinary outbound <a href> links are fine;
//   * the English and Japanese books have the same pages, with the same heading ids.
//
// No dependencies. Usage: node scripts/check-docs-links.mjs [dir]
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(process.argv[2] ?? join(here, '..', 'target', 'docs-site'));

if (!existsSync(root)) {
  console.error(`check-docs-links: ${root} does not exist. Run scripts/build-docs-site.sh first.`);
  process.exit(2);
}

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...walk(path));
    else out.push(path);
  }
  return out;
}

const files = walk(root);
const htmlFiles = files.filter((f) => f.endsWith('.html'));
const cssFiles = files.filter((f) => f.endsWith('.css'));
const problems = [];
const rel = (p) => relative(root, p).split(sep).join('/');
const fail = (file, message) => problems.push(`${rel(file)}: ${message}`);

const isRemote = (url) => /^(https?:)?\/\//i.test(url);
const isSkippable = (url) => /^(mailto:|tel:|javascript:|data:|blob:|#?$)/i.test(url);

// ---- anchors ---------------------------------------------------------------

const anchorCache = new Map();
function anchorsOf(file) {
  if (!anchorCache.has(file)) {
    const text = readFileSync(file, 'utf8');
    const ids = new Set();
    for (const m of text.matchAll(/\b(?:id|name)\s*=\s*"([^"]*)"/g)) ids.add(m[1]);
    for (const m of text.matchAll(/\b(?:id|name)\s*=\s*'([^']*)'/g)) ids.add(m[1]);
    anchorCache.set(file, ids);
  }
  return anchorCache.get(file);
}

/** Resolve an internal URL found in `from` to a file, or null if it is missing. */
function resolveTarget(from, url) {
  const clean = url.split('#')[0].split('?')[0];
  let target = clean === '' ? from : resolve(dirname(from), decodeURIComponent(clean));
  if (clean !== '' && existsSync(target) && statSync(target).isDirectory()) {
    target = join(target, 'index.html');
  }
  return existsSync(target) && statSync(target).isFile() ? target : null;
}

// ---- attribute extraction ------------------------------------------------------

/** All `<tag ... attr="value">` triples. A regex is enough for mdBook's output. */
function* attributes(html) {
  for (const tag of html.matchAll(/<([a-zA-Z][a-zA-Z0-9-]*)\b([^>]*)>/g)) {
    const [, name, rest] = tag;
    for (const attr of rest.matchAll(/\s([a-zA-Z:-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')/g)) {
      yield { tag: name.toLowerCase(), attr: attr[1].toLowerCase(), value: attr[2] ?? attr[3] ?? '' };
    }
  }
}

const LOADS = new Set([
  'script:src',
  'link:href',
  'img:src',
  'img:srcset',
  'source:src',
  'source:srcset',
  'iframe:src',
  'video:src',
  'audio:src',
  'embed:src',
  'object:data',
]);

for (const file of htmlFiles) {
  const html = readFileSync(file, 'utf8');
  const is404 = file.endsWith(`${sep}404.html`);

  for (const { tag, attr, value } of attributes(html)) {
    const key = `${tag}:${attr}`;
    const urls = attr === 'srcset' ? value.split(',').map((s) => s.trim().split(/\s+/)[0]) : [value];

    for (const url of urls) {
      if (!url) continue;

      if (LOADS.has(key)) {
        if (isRemote(url)) {
          fail(file, `loads a remote resource: <${tag} ${attr}="${url}">`);
          continue;
        }
        // Resource links must resolve, except in 404.html, which is served from
        // arbitrary depths with a <base> element.
        if (!is404 && !isSkippable(url) && !resolveTarget(file, url)) {
          fail(file, `missing file: <${tag} ${attr}="${url}">`);
        }
        continue;
      }

      if (key === 'a:href' || key === 'area:href') {
        if (isRemote(url) || isSkippable(url) || is404) continue;
        const target = resolveTarget(file, url);
        if (!target) {
          fail(file, `broken link: ${url}`);
          continue;
        }
        const fragment = url.split('#')[1];
        if (fragment && target.endsWith('.html')) {
          const wanted = decodeURIComponent(fragment);
          if (!anchorsOf(target).has(wanted) && !anchorsOf(target).has(fragment)) {
            fail(file, `missing anchor #${fragment} in ${rel(target)} (from ${url})`);
          }
        }
      }
    }
  }

  // Inline <style> and style="" must not pull anything from the network either.
  for (const m of html.matchAll(/url\(\s*['"]?((?:https?:)?\/\/[^)'"]+)/gi)) {
    fail(file, `inline CSS loads a remote resource: ${m[1]}`);
  }
}

for (const file of cssFiles) {
  const css = readFileSync(file, 'utf8');
  for (const m of css.matchAll(/url\(\s*(['"]?)([^)'"]+)\1\s*\)/gi)) {
    const url = m[2].trim();
    if (isRemote(url)) fail(file, `CSS loads a remote resource: ${url}`);
    else if (!isSkippable(url) && !url.startsWith('#') && !resolveTarget(file, url)) {
      fail(file, `CSS refers to a missing file: ${url}`);
    }
  }
  for (const m of css.matchAll(/@import\s+(?:url\()?\s*['"]?([^'")\s;]+)/gi)) {
    if (isRemote(m[1])) fail(file, `CSS @import loads a remote resource: ${m[1]}`);
  }
}

// ---- the two books must match -----------------------------------------------------

const pagesOf = (lang) =>
  existsSync(join(root, lang))
    ? readdirSync(join(root, lang)).filter((n) => n.endsWith('.html') && n !== '404.html').sort()
    : [];
const en = pagesOf('en');
const ja = pagesOf('ja');
for (const page of en) if (!ja.includes(page)) problems.push(`ja/${page}: exists in en but not in ja`);
for (const page of ja) if (!en.includes(page)) problems.push(`en/${page}: exists in ja but not in en`);

// A link to `page.html#anchor` has to keep working when the reader switches language
// (the switcher carries the #anchor across), so every heading id must exist in both books.
const headingIds = (file) =>
  new Set([...readFileSync(file, 'utf8').matchAll(/<h[1-6]\s+id="([^"]+)"/g)].map((m) => m[1]));
for (const page of en.filter((p) => ja.includes(p))) {
  const a = headingIds(join(root, 'en', page));
  const b = headingIds(join(root, 'ja', page));
  for (const id of a) if (!b.has(id)) problems.push(`ja/${page}: heading #${id} exists in en but not in ja`);
  for (const id of b) if (!a.has(id)) problems.push(`en/${page}: heading #${id} exists in ja but not in en`);
}

// ---- report -----------------------------------------------------------------------------

console.log(
  `checked ${htmlFiles.length} HTML and ${cssFiles.length} CSS files under ${root}` +
    ` (${en.length} English pages, ${ja.length} Japanese pages)`
);
if (problems.length > 0) {
  console.error(`\n${problems.length} problem(s):`);
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}
console.log('all internal links, anchors and resources resolve; nothing is loaded from the network');
