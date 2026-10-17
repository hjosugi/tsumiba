// Static checks on the pages and on what the extension is allowed to do.
// A real browser (test/browser) proves they work; these prove they are wired
// consistently and keep their promises.

import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const src = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'src');
const read = (name) => readFileSync(join(src, name), 'utf8');

function sourceFiles(dir = src) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? sourceFiles(full) : [full];
  });
}

for (const page of ['popup', 'options']) {
  test(`${page}: every element the script looks up exists in the HTML`, () => {
    const html = read(`${page}.html`);
    const ids = new Set([...html.matchAll(/\bid="([\w-]+)"/g)].map((m) => m[1]));
    const wanted = [...read(`${page}.js`).matchAll(/\$\('([\w-]+)'\)/g)].map((m) => m[1]);
    const wantedByTemplate = [...read(`${page}.js`).matchAll(/\$\(`([\w-]+)-error`\)/g)].map((m) => `${m[1]}-error`);
    for (const id of [...wanted, ...wantedByTemplate]) assert.ok(ids.has(id), `${page}.js uses #${id}, which ${page}.html lacks`);
    // showError() builds "<field>-error" ids at run time.
    if (page === 'options') for (const field of ['port', 'token']) assert.ok(ids.has(`${field}-error`));
  });

  test(`${page}: the HTML loads only its own script and stylesheet`, () => {
    const html = read(`${page}.html`);
    assert.match(html, new RegExp(`<script type="module" src="${page}\\.js"></script>`));
    assert.match(html, new RegExp(`<link rel="stylesheet" href="${page}\\.css" />`));
    assert.ok(!/<script(?![^>]*\bsrc=)/.test(html), 'no inline scripts (the default CSP forbids them anyway)');
    assert.ok(!/\son\w+=/.test(html), 'no inline event handlers');
    assert.ok(!/https?:\/\//.test(html), 'no remote resources');
    assert.ok(existsSync(join(src, `${page}.js`)) && existsSync(join(src, `${page}.css`)));
  });

  test(`${page}: has a language, a title, a viewport and scheme support`, () => {
    const html = read(`${page}.html`);
    assert.match(html, /<html lang="en">/);
    assert.match(html, /<title>tsumiba<\/title>/);
    assert.match(html, /name="viewport"/);
    assert.match(html, /name="color-scheme" content="dark light"/);
    assert.match(read(`${page}.css`), /prefers-color-scheme: light/);
  });
}

test('every form control has a label', () => {
  const html = read('options.html');
  for (const id of ['port', 'token']) assert.match(html, new RegExp(`<label for="${id}"`));
  assert.match(read('popup.html'), /<label for="memo"/);
});

test('the toolbar buttons are real <button type=button> elements', () => {
  for (const page of ['popup.html', 'options.html']) {
    const html = read(page);
    for (const m of html.matchAll(/<button\b([^>]*)>/g)) {
      assert.match(m[1], /type="(button|submit)"/, `${page}: <button${m[1]}> needs a type`);
    }
  }
});

test('the extension makes no network calls except to the local app and to a chosen image', () => {
  const fetchers = sourceFiles()
    .filter((f) => /\bfetch(?:Impl)?\(/.test(readFileSync(f, 'utf8').replace(/\/\/.*$/gm, '')))
    .map((f) => relative(src, f).split('\\').join('/'))
    .sort();
  // api.js talks to 127.0.0.1; background.js fetches the image the user picked.
  assert.deepEqual(fetchers, ['background.js', 'lib/api.js']);
});

test('nothing dynamic or remote is loaded', () => {
  for (const file of sourceFiles()) {
    const code = readFileSync(file, 'utf8').replace(/\/\/.*$/gm, '').replace(/\/\*[\s\S]*?\*\//g, '');
    const name = relative(src, file);
    for (const forbidden of [/\beval\s*\(/, /new Function\s*\(/, /importScripts\s*\(/, /XMLHttpRequest/, /WebSocket/, /sendBeacon/, /\bimport\s*\(\s*['"`]https?:/, /from\s+['"]https?:/]) {
      assert.ok(!forbidden.test(code), `${name} uses ${forbidden}`);
    }
  }
});

test('the base URL is built from the loopback address, never from user text', () => {
  const api = read('lib/api.js');
  assert.match(api, /`http:\/\/127\.0\.0\.1:\$\{port\}\/v1`/);
  assert.ok(!/localhost/.test(api.replace(/\/\/.*$/gm, '')), 'only the numeric loopback address is used');
});
