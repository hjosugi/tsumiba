// background.js, loaded with a fake `chrome`, driving the real tsumiba.
//
// The browser's role (menu clicks, the shortcut, the permission prompt, the
// badge) is played by test/helpers/fake-chrome.js; everything else is real:
// the code under test, fetch, and the app.

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import http from 'node:http';
import { after, before, beforeEach, describe, test } from 'node:test';

import { createClient } from '../../src/lib/api.js';
import { imageNote, linkNote, pageNote, selectionNote } from '../../src/lib/format.js';
import { t } from '../../src/lib/i18n.js';
import { createFakeChrome, installChrome, pinLanguage, uninstallChrome } from '../helpers/fake-chrome.js';
import { needsBinary, startServer } from '../helpers/server.js';

// A real 1x1 PNG.
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
  'base64',
);
const JPEG = Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0, 0x10, 0x4a, 0x46, 0x49, 0x46, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0xff, 0xd9]);

const TAB = { id: 7, title: 'The Page 日本語', url: 'https://example.com/article?id=1' };

function startImageServer() {
  const server = http.createServer((req, res) => {
    const path = new URL(req.url, 'http://x').pathname;
    const send = (status, type, body) => {
      res.statusCode = status;
      res.setHeader('Content-Type', type);
      res.end(body);
    };
    if (path === '/pics/cat.png') return send(200, 'image/png', PNG);
    if (path === '/pics/photo.php') return send(200, 'image/jpeg', JPEG);
    if (path === '/pics/%E7%8C%AB.png') return send(200, 'image/png', PNG);
    if (path === '/pics/page.html') return send(200, 'text/html', '<html>please log in</html>');
    if (path === '/pics/big.png') return send(200, 'image/png', Buffer.alloc(2 * 1024 * 1024, 1));
    return send(404, 'text/plain', 'nope');
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server)));
}

describe('background.js', needsBinary, () => {
  let server;
  let images;
  let imageBase;
  let fake;
  let background;
  let blockImages = false;
  let badgeTimers = [];
  const realFetch = globalThis.fetch;
  const realSetTimeout = globalThis.setTimeout;

  before(async () => {
    server = await startServer({ config: '[storage]\nmax_upload_mb = 1\n' });
    images = await startImageServer();
    imageBase = `http://127.0.0.1:${images.address().port}`;

    pinLanguage('en-US');
    fake = createFakeChrome();
    installChrome(fake);

    // Stand-in for "the site does not send CORS headers": a cross-origin image
    // fetch fails until the user has granted access to all sites.
    globalThis.fetch = (url, init) => {
      if (blockImages && String(url).startsWith(imageBase) && !fake.state.granted) {
        return Promise.reject(new TypeError('Failed to fetch'));
      }
      return realFetch(url, init);
    };
    // Catch the timer that clears the badge instead of waiting for it.
    globalThis.setTimeout = (fn, ms, ...rest) => {
      if (ms === 2500) {
        badgeTimers.push(fn);
        return 0;
      }
      return realSetTimeout(fn, ms, ...rest);
    };

    background = await import('../../src/background.js?under-test');
    await background.ensureMenus();
  });

  after(async () => {
    globalThis.fetch = realFetch;
    globalThis.setTimeout = realSetTimeout;
    uninstallChrome();
    images?.close();
    await server?.cleanup();
  });

  beforeEach(() => {
    blockImages = false;
    badgeTimers = [];
    fake.state.storage = { port: server.port, token: server.token };
    fake.state.granted = false;
    fake.state.requestResult = true;
    fake.state.permissionRequests = 0;
    fake.state.optionsOpened = 0;
    fake.state.badge = { text: '', color: null, title: 'tsumiba' };
    fake.state.badgeHistory = [];
  });

  const click = (menuItemId, info = {}, tab = TAB) =>
    fake.contextMenus.onClicked.fire({ menuItemId, pageUrl: tab.url, ...info }, tab);

  const notes = () => server.list().notes;
  const files = () => server.list().files;
  const noteWithBody = (body) => notes().find((n) => n.body === body);
  const imageSrc = (path) => `${imageBase}${path}`;

  // ---- start-up ---------------------------------------------------------------

  test('listeners are registered at import time, once', () => {
    assert.equal(fake.runtime.onInstalled.listeners.length, 1);
    assert.equal(fake.runtime.onStartup.listeners.length, 1);
    assert.equal(fake.contextMenus.onClicked.listeners.length, 1);
    assert.equal(fake.commands.onCommand.listeners.length, 1);
  });

  test('the four menu items exist with the right titles and contexts', () => {
    const menus = fake.state.menus;
    assert.deepEqual([...menus.keys()].sort(), ['tsumiba-image', 'tsumiba-link', 'tsumiba-page', 'tsumiba-selection']);
    assert.equal(menus.get('tsumiba-selection').title, 'Send selection to tsumiba');
    assert.equal(menus.get('tsumiba-link').title, 'Send link to tsumiba');
    assert.equal(menus.get('tsumiba-image').title, 'Send image to tsumiba');
    assert.equal(menus.get('tsumiba-page').title, 'Send page to tsumiba');
    assert.deepEqual(menus.get('tsumiba-selection').contexts, ['selection']);
    assert.deepEqual(menus.get('tsumiba-link').contexts, ['link']);
    assert.deepEqual(menus.get('tsumiba-image').contexts, ['image']);
    assert.deepEqual(menus.get('tsumiba-page').contexts, ['page']);
  });

  test('install, start-up and a direct call arriving together never duplicate a menu', async () => {
    await Promise.all([
      fake.runtime.onInstalled.fire({ reason: 'install' }),
      fake.runtime.onStartup.fire(),
      background.ensureMenus(),
      background.ensureMenus(),
    ]);
    await background.ensureMenus();
    assert.equal(fake.state.menus.size, 4);
    assert.deepEqual(fake.state.menuErrors, [], 'Chrome would have refused a duplicate id');
  });

  // ---- each menu item ---------------------------------------------------------

  test('"Send page" saves the title and address, and flashes a tick', async () => {
    await click('tsumiba-page');
    const expected = pageNote({ title: TAB.title, url: TAB.url });
    assert.equal(expected, '# The Page 日本語\nhttps://example.com/article?id=1\n');
    assert.ok(noteWithBody(expected), 'the note is in the shelf');

    assert.equal(fake.state.badge.text, '✓');
    assert.equal(fake.state.badge.color, '#2e9d57');
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'sentOk')}`);
  });

  test('the badge clears itself after about two and a half seconds', async () => {
    await click('tsumiba-page');
    assert.equal(fake.state.badge.text, '✓');
    assert.equal(badgeTimers.length, 1, 'exactly one clear is scheduled');
    await badgeTimers[0]();
    assert.equal(fake.state.badge.text, '');
    assert.equal(fake.state.badge.title, 'tsumiba');
  });

  test('"Send selection" quotes the text and links its source', async () => {
    const selectionText = 'first line\nsecond line';
    await click('tsumiba-selection', { selectionText });
    const expected = selectionNote({ selection: selectionText, title: TAB.title, url: TAB.url });
    assert.equal(expected, '> first line\n> second line\n\n[The Page 日本語](https://example.com/article?id=1)\n');
    assert.ok(noteWithBody(expected));
    assert.equal(fake.state.badge.text, '✓');
  });

  test('"Send link" saves the target and where it was found (Firefox gives the link text)', async () => {
    await click('tsumiba-link', { linkUrl: 'https://target.example/doc', linkText: 'Read the doc' });
    const expected = linkNote({ href: 'https://target.example/doc', text: 'Read the doc', pageTitle: TAB.title, pageUrl: TAB.url });
    assert.equal(
      expected,
      '# Read the doc\nhttps://target.example/doc\n\nFrom: [The Page 日本語](https://example.com/article?id=1)\n',
    );
    assert.ok(noteWithBody(expected));
  });

  test('"Send link" without link text uses the selection, then the address', async () => {
    await click('tsumiba-link', { linkUrl: 'https://target.example/a', selectionText: 'selected words' });
    assert.ok(noteWithBody(linkNote({ href: 'https://target.example/a', text: 'selected words', pageTitle: TAB.title, pageUrl: TAB.url })));
    await click('tsumiba-link', { linkUrl: 'https://target.example/b' });
    assert.ok(noteWithBody(linkNote({ href: 'https://target.example/b', text: '', pageTitle: TAB.title, pageUrl: TAB.url })));
  });

  test('the keyboard shortcut sends the page it is pressed on', async () => {
    const tab = { id: 8, title: 'Shortcut Page', url: 'https://shortcut.example/' };
    await fake.commands.onCommand.fire('send-page', tab);
    assert.ok(noteWithBody(pageNote({ title: tab.title, url: tab.url })));
    assert.equal(fake.state.badge.text, '✓');
  });

  test('the shortcut falls back to the active tab when the browser passes none', async () => {
    fake.state.activeTab = { id: 9, title: 'Active Fallback', url: 'https://fallback.example/' };
    await fake.commands.onCommand.fire('send-page');
    assert.ok(noteWithBody(pageNote({ title: 'Active Fallback', url: 'https://fallback.example/' })));
  });

  test('other commands are ignored', async () => {
    const before = notes().length;
    await fake.commands.onCommand.fire('something-else', TAB);
    assert.equal(notes().length, before);
    assert.equal(fake.state.badgeHistory.length, 0);
  });

  test('unknown menu items are ignored', async () => {
    const before = notes().length;
    await click('not-ours');
    assert.equal(notes().length, before);
  });

  // ---- images -------------------------------------------------------------------

  test('"Send image" uploads the bytes as a file named from the address, sourced from the page', async () => {
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/cat.png') });
    const file = files().find((f) => f.name === 'cat.png' && f.source === TAB.url);
    assert.ok(file, JSON.stringify(files().map((f) => [f.name, f.source])));
    assert.deepEqual(readFileSync(file.path), PNG);
    assert.equal(file.size, PNG.length);
    assert.equal(fake.state.badge.text, '✓');
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'sentImageFile')}`);
    assert.equal(fake.state.permissionRequests, 0, 'no permission was needed');
  });

  test('image names are decoded and corrected from the content type', async () => {
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/%E7%8C%AB.png') });
    assert.ok(files().find((f) => f.name === '猫.png'));
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/photo.php?id=3') });
    const photo = files().find((f) => f.name === 'photo.jpg');
    assert.ok(photo, 'photo.php served as image/jpeg is saved as photo.jpg');
    assert.deepEqual(readFileSync(photo.path), JPEG);
  });

  test('a data: image is uploaded without any permission', async () => {
    blockImages = true;
    await click('tsumiba-image', { srcUrl: `data:image/png;base64,${PNG.toString('base64')}` });
    const file = files().find((f) => f.name === 'image.png');
    assert.ok(file);
    assert.deepEqual(readFileSync(file.path), PNG);
    assert.equal(fake.state.permissionRequests, 0);
  });

  test('when the fetch is blocked, it asks once for access to all sites and then succeeds', async () => {
    blockImages = true;
    fake.state.requestResult = true;
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/cat.png') });
    assert.equal(fake.state.permissionRequests, 1);
    const file = files().filter((f) => f.name === 'cat.png' && f.source === TAB.url).at(-1);
    assert.ok(file);
    assert.deepEqual(readFileSync(file.path), PNG);
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'sentImageFile')}`);
    assert.equal(fake.state.storage.imagePermissionDenied, false);

    // Now that access is granted nothing is asked again.
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/cat.png') });
    assert.equal(fake.state.permissionRequests, 1);
  });

  test('if access is refused, the image address is saved as a note, and the user is not asked again', async () => {
    blockImages = true;
    fake.state.requestResult = false;
    const src = imageSrc('/pics/cat.png');
    await click('tsumiba-image', { srcUrl: src });

    const expected = imageNote({ src, fileName: 'cat.png', pageTitle: TAB.title, pageUrl: TAB.url });
    assert.ok(noteWithBody(expected), 'the fallback note is in the shelf');
    assert.equal(fake.state.permissionRequests, 1);
    assert.equal(fake.state.storage.imagePermissionDenied, true, 'the refusal is remembered');
    assert.equal(fake.state.badge.text, '✓', 'the address was saved, so this is not a failure');
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'sentImageLink')}`);

    await click('tsumiba-image', { srcUrl: src });
    assert.equal(fake.state.permissionRequests, 1, 'no second prompt');
  });

  test('access that is already granted is not asked for again when the image is simply unreachable', async () => {
    fake.state.granted = true;
    // Point at a port nobody listens on.
    const src = 'http://127.0.0.1:1/unreachable.png';
    await click('tsumiba-image', { srcUrl: src });
    assert.equal(fake.state.permissionRequests, 0);
    assert.ok(noteWithBody(imageNote({ src, fileName: 'unreachable.png', pageTitle: TAB.title, pageUrl: TAB.url })));
  });

  test('an HTTP error or a non-image reply falls back to saving the address, with no prompt', async () => {
    for (const path of ['/pics/missing.png', '/pics/page.html']) {
      fake.state.permissionRequests = 0;
      const src = imageSrc(path);
      await click('tsumiba-image', { srcUrl: src });
      const name = path.split('/').pop();
      assert.ok(noteWithBody(imageNote({ src, fileName: name, pageTitle: TAB.title, pageUrl: TAB.url })), path);
      assert.equal(fake.state.permissionRequests, 0, path);
    }
  });

  test('a blob: image cannot be fetched from here and never triggers a prompt', async () => {
    const src = 'blob:https://example.com/0b8c6f0e-0000-4000-8000-000000000000';
    await click('tsumiba-image', { srcUrl: src });
    assert.equal(fake.state.permissionRequests, 0);
    assert.ok(noteWithBody(imageNote({ src, fileName: 'image', pageTitle: TAB.title, pageUrl: TAB.url })));
  });

  test('an image over the upload limit is reported as too large', async () => {
    const before = files().length;
    await click('tsumiba-image', { srcUrl: imageSrc('/pics/big.png') });
    assert.equal(files().length, before);
    assert.equal(fake.state.badge.text, '!');
    assert.equal(fake.state.badge.color, '#d6453d');
    // A newer app announces its limit (max_upload_mb = 1 here) and the message
    // says so; an older one is only known by its 413.
    const announced = await createClient({ port: server.port, token: server.token }).maxUploadBytes();
    const message = announced ? t('en', 'errTooLargeLimit', { limit: 1 }) : t('en', 'errTooLarge');
    assert.equal(fake.state.badge.title, `tsumiba — ${message}`);
  });

  /** A stand-in for tsumiba's API that records what reaches /files. */
  async function stubApp({ announceLimitMb }) {
    const hits = { status: 0, files: 0 };
    const stub = http.createServer((req, res) => {
      const path = new URL(req.url, 'http://x').pathname;
      const reply = (status, body) => {
        res.statusCode = status;
        res.setHeader('Content-Type', 'application/json');
        res.end(JSON.stringify(body));
      };
      if (path === '/v1/status') {
        hits.status += 1;
        return reply(200, announceLimitMb ? { notes: 0, files: 0, visible: false, max_upload_mb: announceLimitMb } : { notes: 0, files: 0, visible: false });
      }
      if (path === '/v1/files') {
        hits.files += 1;
        req.resume();
        // What an app without the announcement does with an oversized upload.
        return req.on('end', () => reply(announceLimitMb ? 201 : 413, announceLimitMb ? { id: 'f' } : { error: 'too large' }));
      }
      return reply(404, { error: 'not found' });
    });
    await new Promise((r) => stub.listen(0, '127.0.0.1', r));
    return { hits, port: stub.address().port, close: () => new Promise((r) => stub.close(r)) };
  }

  test('with an announced limit, an oversized image is refused locally and never sent', async () => {
    const stub = await stubApp({ announceLimitMb: 1 });
    try {
      fake.state.storage = { port: stub.port, token: server.token };
      await click('tsumiba-image', { srcUrl: imageSrc('/pics/big.png') });
      assert.equal(stub.hits.files, 0, 'the 2 MiB body never left the browser');
      assert.equal(stub.hits.status, 1);
      assert.equal(fake.state.badge.text, '!');
      assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'errTooLargeLimit', { limit: 1 })}`);

      // A small image still goes through, after the same one status check.
      await click('tsumiba-image', { srcUrl: imageSrc('/pics/cat.png') });
      assert.equal(stub.hits.files, 1);
      assert.equal(fake.state.badge.text, '✓');
    } finally {
      await stub.close();
    }
  });

  test('without an announced limit the upload is attempted, and the 413 is reported', async () => {
    const stub = await stubApp({ announceLimitMb: 0 });
    try {
      fake.state.storage = { port: stub.port, token: server.token };
      await click('tsumiba-image', { srcUrl: imageSrc('/pics/big.png') });
      assert.equal(stub.hits.files, 1);
      assert.equal(fake.state.badge.text, '!');
      assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'errTooLarge')}`);
    } finally {
      await stub.close();
    }
  });

  // ---- when something is wrong ----------------------------------------------------

  test('without a token nothing is sent, the badge says so, and the settings page opens', async () => {
    fake.state.storage = { port: server.port, token: '' };
    const before = notes().length;
    await click('tsumiba-page');
    assert.equal(notes().length, before);
    assert.equal(fake.state.badge.text, '!');
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'errNotPaired')}`);
    assert.equal(fake.state.optionsOpened, 1);
  });

  test('a wrong token is reported as such', async () => {
    fake.state.storage = { port: server.port, token: 'f'.repeat(64) };
    const before = notes().length;
    await click('tsumiba-page');
    assert.equal(notes().length, before);
    assert.equal(fake.state.badge.text, '!');
    assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'errUnauthorized')}`);
    assert.equal(fake.state.optionsOpened, 0, 'the page only opens when there is no token at all');
  });

  test('an empty selection or a link without an address is refused, not sent', async () => {
    const before = notes().length;
    await click('tsumiba-selection', { selectionText: '   \n ' });
    assert.equal(fake.state.badge.text, '!');
    await click('tsumiba-link', {});
    assert.equal(fake.state.badge.text, '!');
    assert.equal(notes().length, before);
  });

  test('when the app is not running every menu item fails gracefully', async () => {
    const down = await startServer();
    const { port, token } = down;
    await down.stop();
    fake.state.storage = { port, token };

    for (const [id, info] of [
      ['tsumiba-page', {}],
      ['tsumiba-selection', { selectionText: 'x' }],
      ['tsumiba-link', { linkUrl: 'https://t.example/' }],
      ['tsumiba-image', { srcUrl: imageSrc('/pics/cat.png') }],
    ]) {
      fake.state.badge = { text: '', color: null, title: 'tsumiba' };
      await click(id, info); // must resolve, not throw
      assert.equal(fake.state.badge.text, '!', id);
      assert.equal(fake.state.badge.title, `tsumiba — ${t('en', 'errNotRunning')}`, id);
    }
    await fake.commands.onCommand.fire('send-page', TAB);
    assert.equal(fake.state.badge.text, '!');
    await down.cleanup();
  });
});

describe('background.js in Japanese', needsBinary, () => {
  test('the menus and messages follow navigator.language', async () => {
    pinLanguage('ja-JP');
    const fake = createFakeChrome();
    installChrome(fake);
    try {
      const background = await import('../../src/background.js?japanese');
      await background.ensureMenus();
      assert.equal(fake.state.menus.get('tsumiba-page').title, 'ページを tsumiba に送る');
      assert.equal(fake.state.menus.get('tsumiba-image').title, '画像を tsumiba に送る');
    } finally {
      uninstallChrome();
      pinLanguage('en-US');
    }
  });
});
