import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { TsumibaError } from '../../src/lib/api.js';
import { en, errorMessage, ja, pickLang, placeholders, t } from '../../src/lib/i18n.js';

const src = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'src');

function sourceFiles(dir = src) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? sourceFiles(full) : [full];
  });
}

test('English and Japanese have exactly the same keys', () => {
  assert.deepEqual(Object.keys(ja).sort(), Object.keys(en).sort());
});

test('no string is empty or left as a placeholder', () => {
  for (const [lang, table] of [['en', en], ['ja', ja]]) {
    for (const [key, value] of Object.entries(table)) {
      assert.ok(typeof value === 'string' && value.trim() !== '', `${lang}.${key} is empty`);
      assert.ok(!/TODO|FIXME|XXX/.test(value), `${lang}.${key} looks unfinished`);
    }
  }
});

test('both languages use the same {placeholders} in every string', () => {
  for (const key of Object.keys(en)) {
    assert.deepEqual(placeholders(ja[key]), placeholders(en[key]), `placeholders of "${key}" differ`);
  }
});

test('Japanese strings are really translated', () => {
  // A copy-pasted English string would pass every other check.
  const untranslated = Object.keys(en).filter((k) => ja[k] === en[k]);
  assert.deepEqual(untranslated, []);
});

test('every key the code and the pages ask for exists', () => {
  const wanted = new Map(); // key -> file
  for (const file of sourceFiles()) {
    const text = readFileSync(file, 'utf8');
    for (const m of text.matchAll(/data-i18n(?:-[a-z-]+)?="(\w+)"/g)) wanted.set(m[1], file);
    for (const m of text.matchAll(/\bt\(lang(?:\(\))?, '(\w+)'/g)) wanted.set(m[1], file);
    for (const m of text.matchAll(/(?:title|tone|detail):? '(\w+)'/g)) {
      if (m[1] in en) wanted.set(m[1], file);
    }
  }
  for (const [key, file] of wanted) assert.ok(key in en, `"${key}" (used in ${file}) is not defined`);
});

test('no string is defined but never used', () => {
  // Everything except the two dictionaries themselves (i18n.js also contains the
  // code that looks keys up, which counts as a use).
  const i18n = readFileSync(join(src, 'lib', 'i18n.js'), 'utf8');
  const code = i18n.slice(i18n.indexOf('export const languages'));
  const everything = [
    code,
    ...sourceFiles()
      .filter((f) => !f.endsWith(join('lib', 'i18n.js')))
      .map((f) => readFileSync(f, 'utf8')),
  ].join('\n');
  const unused = Object.keys(en).filter((key) => !new RegExp(`['"]${key}['"]`).test(everything));
  assert.deepEqual(unused, []);
});

test('the language follows navigator.language', () => {
  for (const tag of ['ja', 'ja-JP', 'JA', 'ja_JP', 'ja-Jpan-JP']) assert.equal(pickLang(tag), 'ja', tag);
  for (const tag of ['en', 'en-US', 'de-DE', 'zh-CN', '', undefined, null, 'jam']) assert.equal(pickLang(tag), 'en', String(tag));
});

test('t fills placeholders, leaves unknown ones, and falls back to English then the key', () => {
  assert.equal(t('en', 'statusOkDetail', { version: '1.2', notes: 3, files: 4 }), 'v1.2 · Notes: 3 · Files: 4');
  assert.equal(t('ja', 'statusOkDetail', { version: '1.2', notes: 3, files: 4 }), 'v1.2 · メモ 3 件 · ファイル 4 件');
  assert.equal(t('en', 'statusNotTsumibaDetail'), 'Another program uses port {port}.');
  assert.equal(t('fr', 'save'), 'Save');
  assert.equal(t('en', 'no.such.key'), 'no.such.key');
});

test('every error code has a message in both languages', () => {
  const codes = ['not-running', 'unauthorized', 'too-large', 'forbidden', 'not-tsumiba', 'bad-request', 'not-paired', 'other'];
  for (const lang of ['en', 'ja']) {
    const seen = new Set();
    for (const code of codes) {
      const message = errorMessage(lang, new TsumibaError(code, 'detail text'));
      assert.ok(message && !/\{\w+\}/.test(message), `${lang}/${code}: ${message}`);
      seen.add(message);
    }
    assert.equal(seen.size, codes.length, `${lang}: each code reads differently`);
  }
  // When the limit is known the message says what it is.
  const limited = new TsumibaError('too-large', 'x', { limitBytes: 512 * 1024 * 1024 });
  assert.match(errorMessage('en', limited), /512 MiB/);
  assert.match(errorMessage('ja', limited), /512 MiB/);
  assert.notEqual(errorMessage('en', limited), errorMessage('en', new TsumibaError('too-large', 'x')));
  assert.match(errorMessage('en', new TsumibaError('other', 'boom')), /boom/);
  assert.match(errorMessage('en', new Error('plain')), /plain/);
});
