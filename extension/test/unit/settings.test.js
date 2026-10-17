import assert from 'node:assert/strict';
import { afterEach, beforeEach, test } from 'node:test';

import {
  DEFAULTS,
  DEFAULT_PORT,
  isValidToken,
  loadSettings,
  normalizeToken,
  parsePort,
  saveSettings,
} from '../../src/lib/settings.js';
import { createFakeChrome, installChrome, uninstallChrome } from '../helpers/fake-chrome.js';

const TOKEN = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';

let fake;
beforeEach(() => {
  fake = createFakeChrome();
  installChrome(fake);
});
afterEach(uninstallChrome);

test('the default port matches the app', () => {
  assert.equal(DEFAULT_PORT, 47821);
  assert.equal(DEFAULTS.port, 47821);
});

test('ports must be whole numbers from 1024 to 65535', () => {
  for (const good of ['1024', 1024, '47821', ' 65535 ', 8080]) assert.ok(parsePort(good) !== null, String(good));
  for (const bad of ['', 'abc', '1023', '65536', '0', '-1', '12.5', '1e3', '47821x', null, undefined, '999999']) {
    assert.equal(parsePort(bad), null, String(bad));
  }
  assert.equal(parsePort(' 47821 '), 47821);
});

test('pasted tokens are tidied: whitespace removed, lowercased', () => {
  assert.equal(normalizeToken(`  ${TOKEN.toUpperCase()}\n`), TOKEN);
  assert.equal(normalizeToken(`${TOKEN.slice(0, 32)}\n${TOKEN.slice(32)}`), TOKEN);
  assert.equal(normalizeToken(undefined), '');
});

test('only 64 lowercase hex digits are a token', () => {
  assert.ok(isValidToken(TOKEN));
  for (const bad of ['', 'abc', TOKEN.slice(1), `${TOKEN}0`, TOKEN.toUpperCase(), `${TOKEN.slice(1)}g`]) {
    assert.ok(!isValidToken(bad), bad);
  }
});

test('with nothing stored the defaults apply', async () => {
  assert.deepEqual(await loadSettings(), { port: 47821, token: '', imagePermissionDenied: false });
});

test('saved settings are read back', async () => {
  await saveSettings({ port: '5000', token: ` ${TOKEN.toUpperCase()} ` });
  assert.deepEqual(await loadSettings(), { port: 5000, token: TOKEN, imagePermissionDenied: false });
});

test('a partial save keeps the rest', async () => {
  await saveSettings({ port: 5000, token: TOKEN });
  await saveSettings({ imagePermissionDenied: true });
  assert.deepEqual(await loadSettings(), { port: 5000, token: TOKEN, imagePermissionDenied: true });
});

test('unusable values are refused and nothing is written', async () => {
  await assert.rejects(saveSettings({ port: 80 }), RangeError);
  await assert.rejects(saveSettings({ port: 'x' }), RangeError);
  await assert.rejects(saveSettings({ token: 'not-a-token' }), TypeError);
  assert.deepEqual(fake.state.storage, {});
});

test('clearing the token is allowed', async () => {
  await saveSettings({ token: TOKEN });
  await saveSettings({ token: '' });
  assert.equal((await loadSettings()).token, '');
});

test('damaged stored values fall back to defaults instead of breaking the popup', async () => {
  fake.state.storage = { port: 'banana', token: 12345, imagePermissionDenied: 'yes' };
  assert.deepEqual(await loadSettings(), { port: 47821, token: '', imagePermissionDenied: false });
});
