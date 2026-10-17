// The extension in a real Chromium, talking to the real tsumiba.
//
// Run with `npm run test:browser`. Needs a Chromium on PATH (or $CHROMIUM_BIN) and a tsumiba binary; otherwise
// the whole file is skipped. Branded Google Chrome is not used on purpose: it
// has not accepted --load-extension since version 137. Run it with, say,
//
//   nix shell nixpkgs#chromium -c npm run test:browser
//
// The browser runs headless, driven over the DevTools Protocol with no
// dependencies (test/helpers/cdp.js).

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { accessSync, constants, existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { after, before, describe, test } from 'node:test';

import { build } from '../../build.mjs';
import { pageNote, selectionNote } from '../../src/lib/format.js';
import { Cdp, waitFor } from '../helpers/cdp.js';
import { SKIP_REASON, startServer, tsumibaBinary } from '../helpers/server.js';

function chromiumBinary() {
  if (process.env.CHROMIUM_BIN) return process.env.CHROMIUM_BIN;
  for (const dir of (process.env.PATH ?? '').split(delimiter)) {
    // Not `chromium-browser`: on Ubuntu that is a snap launcher that does not
    // load unpacked extensions.
    for (const name of ['chromium']) {
      const candidate = join(dir, name);
      try {
        accessSync(candidate, constants.X_OK);
        return candidate;
      } catch {
        // keep looking
      }
    }
  }
  return null;
}

const chromium = chromiumBinary();
const skip = !chromium
  ? 'no Chromium found (put `chromium` on PATH or set CHROMIUM_BIN)'
  : !tsumibaBinary()
    ? SKIP_REASON
    : false;

// A real 1x1 PNG.
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
  'base64',
);

function freePort() {
  return new Promise((resolve) => {
    const probe = net.createServer();
    probe.listen(0, '127.0.0.1', () => {
      const { port } = probe.address();
      probe.close(() => resolve(port));
    });
  });
}

describe('the extension in Chromium', { skip, timeout: 240000 }, () => {
  let server;
  let fixture;
  let fixtureOrigin;
  let fixtureUrl;
  let distDir;
  let profile;
  let child;
  let cdp;
  let extensionId;
  let chromiumLog = '';

  const extensionUrl = (page) => `chrome-extension://${extensionId}/${page}`;

  async function targets() {
    return (await cdp.send('Target.getTargets')).targetInfos;
  }

  /** Open a URL in a new tab and attach to it. */
  async function openTab(url) {
    const { targetId } = await cdp.send('Target.createTarget', { url });
    const session = await cdp.attach(targetId);
    await session.send('Page.enable');
    await waitFor(async () => (await session.evaluate('document.readyState')) === 'complete', {
      message: `${url} to load`,
    });
    return session;
  }

  async function closeTab(session) {
    await cdp.send('Target.closeTarget', { targetId: session.targetId }).catch(() => {});
  }

  /** Save settings as the options page would (through an extension page). */
  async function pair(session, settings) {
    await session.evaluate(`chrome.storage.local.clear().then(() => chrome.storage.local.set(${JSON.stringify(settings)}))`);
  }

  /**
   * Open the popup with `fixtureTab` as the active tab, the way it looks when
   * a user clicks the toolbar button on that page. (The popup asks which tab
   * is active when it starts, so make the page active and then reload it.)
   */
  async function openPopup(fixtureTab) {
    const popup = await openTab(extensionUrl('popup.html'));
    await cdp.send('Target.activateTarget', { targetId: fixtureTab.targetId });
    await popup.send('Page.reload');
    await waitFor(async () => (await popup.evaluate('document.readyState')) === 'complete', { message: 'popup reload' });
    await waitFor(async () => (await popup.evaluate(`document.getElementById('status').dataset.state`)) !== 'checking', {
      message: 'the popup to finish checking the connection',
    });
    return popup;
  }

  /**
   * What went wrong in a page, minus the console lines Chromium writes for the
   * failures these tests cause on purpose (wrong token, nothing listening).
   */
  const unexpected = (session) => session.problems.filter((p) => !/Failed to load resource: .*\/v1\//.test(p));

  const text = (session, selector) => session.evaluate(`document.querySelector(${JSON.stringify(selector)}).textContent`);

  before(async () => {
    server = await startServer();

    fixture = http.createServer((req, res) => {
      const path = new URL(req.url, 'http://x').pathname;
      if (path === '/cat.png') {
        res.setHeader('Content-Type', 'image/png');
        return res.end(PNG);
      }
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.end(
        '<!doctype html><meta charset="utf-8"><title>Fixture Page 日本語</title>' +
          '<p id="t">Some selected words<br>and a second line</p>' +
          '<a id="l" href="/elsewhere">a link</a> <img id="i" src="/cat.png">',
      );
    });
    await new Promise((r) => fixture.listen(0, '127.0.0.1', r));
    fixtureOrigin = `http://127.0.0.1:${fixture.address().port}`;
    fixtureUrl = `${fixtureOrigin}/page.html`;

    distDir = mkdtempSync(join(tmpdir(), 'tsumiba-ext-browser-'));
    build({ dist: distDir, zip: false });
    const extensionDir = join(distDir, 'chrome');

    profile = mkdtempSync(join(tmpdir(), 'tsumiba-chromium-'));
    child = spawn(
      chromium,
      [
        '--headless=new',
        '--no-sandbox',
        '--disable-gpu',
        '--no-first-run',
        '--no-default-browser-check',
        `--user-data-dir=${profile}`,
        '--remote-debugging-port=0',
        `--load-extension=${extensionDir}`,
        `--disable-extensions-except=${extensionDir}`,
        'about:blank',
      ],
      { stdio: ['ignore', 'ignore', 'pipe'] },
    );
    child.stderr.on('data', (d) => (chromiumLog += d));

    const port = await waitFor(
      () => {
        const file = join(profile, 'DevToolsActivePort');
        return existsSync(file) ? Number(readFileSync(file, 'utf8').split('\n')[0]) : 0;
      },
      { timeout: 30000, message: 'Chromium to open its DevTools port' },
    );
    const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
    cdp = await Cdp.connect(version.webSocketDebuggerUrl);

    const worker = await waitFor(
      async () => (await targets()).find((t) => t.type === 'service_worker' && t.url.endsWith('/background.js')),
      { timeout: 30000, message: `the extension's service worker to start\n${chromiumLog}` },
    );
    extensionId = new URL(worker.url).host;
  });

  after(async () => {
    try {
      await cdp?.send('Browser.close');
    } catch {
      // already gone
    }
    cdp?.close();
    child?.kill('SIGKILL');
    fixture?.close();
    await server?.cleanup();
    for (const dir of [distDir, profile]) if (dir) rmSync(dir, { recursive: true, force: true });
  });

  test('the service worker runs, registers its listeners, and has created every menu item', async () => {
    const worker = (await targets()).find((t) => t.type === 'service_worker' && t.url.endsWith('/background.js'));
    const session = await cdp.attach(worker.targetId);
    // The worker's JavaScript context appears a moment after attaching.
    await waitFor(async () => (await session.evaluate('typeof chrome?.runtime?.getManifest')) === 'function', {
      message: 'the service worker context',
    });
    // `update` fails for an id that does not exist and changes nothing for one
    // that does, so it is a side-effect-free way to ask "is this menu there?".
    const menuMissing = (id) =>
      session.evaluate(`new Promise((resolve) =>
        chrome.contextMenus.update(${JSON.stringify(id)}, { enabled: true }, () => resolve(chrome.runtime.lastError?.message ?? null)))`);
    const ids = ['tsumiba-selection', 'tsumiba-link', 'tsumiba-image', 'tsumiba-page'];
    // The worker rebuilds its menus when it starts; give that a moment.
    const menus = await waitFor(
      async () => {
        const results = await Promise.all(ids.map(menuMissing));
        return results.every((r) => r === null) ? results : null;
      },
      { message: 'all four context menu items to exist' },
    );
    const info = await session.evaluate(`(async () => ({
      name: chrome.runtime.getManifest().name,
      version: chrome.runtime.getManifest().version,
      listeners: {
        menuClick: chrome.contextMenus.onClicked.hasListeners(),
        command: chrome.commands.onCommand.hasListeners(),
        installed: chrome.runtime.onInstalled.hasListeners(),
      },
      commands: (await chrome.commands.getAll()).map((c) => ({ name: c.name, shortcut: c.shortcut })),
    }))()`);
    assert.equal(menus.length, 4);
    assert.equal(info.name, 'tsumiba');
    assert.match(info.version, /^\d+\.\d+\.\d+$/);
    assert.deepEqual(info.listeners, { menuClick: true, command: true, installed: true });
    // Chrome only assigns a suggested key that does not collide with one of its own
    // (Alt+Shift+T, for example, is "focus the toolbar"), so this also guards the choice.
    assert.ok(info.commands.some((c) => c.name === 'send-page' && /Alt\+Shift\+S/.test(c.shortcut)), JSON.stringify(info.commands));
    assert.deepEqual(unexpected(session), []);
  });

  test('options: loads cleanly, validates, pairs, and tests the connection', async () => {
    const page = await openTab(extensionUrl('options.html'));
    try {
      await pair(page, {});
      await page.send('Page.reload');
      await waitFor(async () => (await page.evaluate(`document.getElementById('port').value`)) === '47821', {
        message: 'the default port to appear',
      });
      assert.equal(await page.evaluate('document.documentElement.lang'), 'en');
      assert.match(await text(page, '#images-state'), /Not allowed/);
      assert.equal(await page.evaluate(`document.getElementById('images-grant').hidden`), false);
      assert.equal(await page.evaluate(`document.getElementById('images-revoke').hidden`), true);

      const submit = () => page.evaluate(`document.getElementById('form').requestSubmit()`);
      const setField = (id, value) =>
        page.evaluate(`(() => { const f = document.getElementById(${JSON.stringify(id)}); f.value = ${JSON.stringify(value)}; f.dispatchEvent(new Event('input', { bubbles: true })); })()`);
      const visible = (id) => page.evaluate(`!document.getElementById(${JSON.stringify(id)}).hidden`);

      // Validation.
      await setField('port', '80');
      await submit();
      assert.equal(await visible('port-error'), true);
      assert.match(await text(page, '#port-error'), /1024/);
      assert.equal(await page.evaluate(`document.getElementById('port').getAttribute('aria-invalid')`), 'true');
      await setField('port', String(server.port));
      await setField('token', 'zz-not-a-token');
      await submit();
      assert.equal(await visible('token-error'), true);
      assert.deepEqual(await page.evaluate('chrome.storage.local.get()'), {}, 'nothing was saved');

      // A token the app does not know.
      await setField('token', 'a'.repeat(64));
      await page.evaluate(`document.getElementById('test').click()`);
      await waitFor(async () => /does not accept this token/.test(await text(page, '#test-result')), { message: 'the wrong-token diagnosis' });
      assert.equal(await page.evaluate(`document.getElementById('test-result').dataset.state`), 'bad');

      // The right one, pasted with whitespace and in capitals.
      await setField('token', `  ${server.token.toUpperCase()}\n`);
      await submit();
      await waitFor(async () => /Connected to tsumiba/.test(await text(page, '#test-result')), { message: 'the success message' });
      assert.equal(await page.evaluate(`document.getElementById('test-result').dataset.state`), 'ok');
      assert.equal(await page.evaluate(`document.getElementById('saved').hidden`), false);
      assert.equal(await page.evaluate(`document.getElementById('token').value`), server.token, 'the field shows the tidied token');
      assert.deepEqual(await page.evaluate('chrome.storage.local.get()'), { port: server.port, token: server.token });

      // The token field is masked until asked.
      assert.equal(await page.evaluate(`document.getElementById('token').type`), 'password');
      await page.evaluate(`document.getElementById('toggle-token').click()`);
      assert.equal(await page.evaluate(`document.getElementById('token').type`), 'text');
      assert.equal(await page.evaluate(`document.getElementById('toggle-token').getAttribute('aria-pressed')`), 'true');
      await page.evaluate(`document.getElementById('toggle-token').click()`);
      assert.equal(await page.evaluate(`document.getElementById('token').type`), 'password');

      // A port nothing listens on.
      await setField('port', String(await freePort()));
      await page.evaluate(`document.getElementById('test').click()`);
      await waitFor(async () => /isn't running/.test(await text(page, '#test-result')), { message: 'the not-running diagnosis' });

      // Settings survive a reload and are tested on load.
      await pair(page, { port: server.port, token: server.token });
      await page.send('Page.reload');
      await waitFor(async () => /Connected to tsumiba/.test(await text(page, '#test-result')), { message: 'the automatic test after a reload' });
      assert.equal(await page.evaluate(`document.getElementById('port').value`), String(server.port));

      assert.deepEqual(unexpected(page), []);
      // ...and the two deliberate failures above did show up in the console.
      assert.equal(page.problems.length, 2, page.problems.join('\n'));
    } finally {
      await closeTab(page);
    }
  });

  test('popup: shows the connection, saves memos (button and shortcut), the page and the selection', async () => {
    const admin = await openTab(extensionUrl('options.html'));
    const tab = await openTab(fixtureUrl);
    try {
      await pair(admin, { port: server.port, token: server.token });
      const selected = await tab.evaluate(`(() => { getSelection().selectAllChildren(document.getElementById('t')); return getSelection().toString(); })()`);
      assert.match(selected, /Some selected words\nand a second line/);

      // The state, at a glance.
      let popup = await openPopup(tab);
      assert.equal(await popup.evaluate(`document.getElementById('status').dataset.state`), 'ok');
      assert.equal(await text(popup, '#status-title'), 'Connected');
      assert.match(await text(popup, '#status-detail'), /^v\d+\.\d+\.\d+ · Notes: \d+ · Files: \d+$/);
      assert.equal(await popup.evaluate(`document.getElementById('save-selection').disabled`), false, 'there is a selection');
      assert.equal(await popup.evaluate(`document.getElementById('memo-hint').textContent`), 'Ctrl+Enter to save');
      assert.equal(await popup.evaluate('document.activeElement.id'), 'memo', 'the memo box has the focus');

      // A memo, with the button.
      await popup.evaluate(`document.getElementById('memo').value = 'memo by button 日本語\\n\\n'`);
      await popup.evaluate(`document.getElementById('save-memo').click()`);
      await waitFor(() => server.list().notes.find((n) => n.body === 'memo by button 日本語\n'), { message: 'the memo' });
      assert.deepEqual(unexpected(popup), []);
      await closeTab(popup);

      // A memo, with Ctrl+Enter.
      popup = await openPopup(tab);
      await popup.evaluate(`(() => {
        const memo = document.getElementById('memo');
        memo.value = 'memo by shortcut';
        memo.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true }));
      })()`);
      await waitFor(() => server.list().notes.find((n) => n.body === 'memo by shortcut\n'), { message: 'the shortcut memo' });
      await closeTab(popup);

      // An empty memo is refused, not sent.
      popup = await openPopup(tab);
      const before = server.list().notes.length;
      await popup.evaluate(`document.getElementById('save-memo').click()`);
      await waitFor(async () => /Nothing to save/.test(await text(popup, '#message')), { message: 'the empty-memo message' });
      assert.equal(server.list().notes.length, before);
      await closeTab(popup);

      // The page.
      popup = await openPopup(tab);
      await popup.evaluate(`document.getElementById('save-page').click()`);
      const pageBody = pageNote({ title: 'Fixture Page 日本語', url: fixtureUrl });
      await waitFor(() => server.list().notes.find((n) => n.body === pageBody), { message: `the page note ${JSON.stringify(pageBody)}` });
      await closeTab(popup);

      // The selection.
      popup = await openPopup(tab);
      await popup.evaluate(`document.getElementById('save-selection').click()`);
      const selectionBody = selectionNote({ selection: selected, title: 'Fixture Page 日本語', url: fixtureUrl });
      await waitFor(() => server.list().notes.find((n) => n.body === selectionBody), { message: `the selection note ${JSON.stringify(selectionBody)}` });
      await closeTab(popup);
    } finally {
      await closeTab(tab);
      await closeTab(admin);
    }
  });

  test('popup: says what is wrong when not paired, when the token is wrong, and when the app is not running', async () => {
    const admin = await openTab(extensionUrl('options.html'));
    const tab = await openTab(fixtureUrl);
    try {
      const state = async (settings) => {
        await pair(admin, settings);
        const popup = await openPopup(tab);
        const view = {
          state: await popup.evaluate(`document.getElementById('status').dataset.state`),
          title: await text(popup, '#status-title'),
          memoDisabled: await popup.evaluate(`document.getElementById('save-memo').disabled`),
          problems: unexpected(popup),
        };
        return { popup, view };
      };

      let { popup, view } = await state({ port: server.port, token: '' });
      assert.deepEqual([view.state, view.title, view.memoDisabled], ['warn', 'Not paired yet', true]);
      assert.deepEqual(view.problems, []);
      await closeTab(popup);

      ({ popup, view } = await state({ port: server.port, token: 'b'.repeat(64) }));
      assert.deepEqual([view.state, view.title], ['bad', "The token doesn't match"]);
      await closeTab(popup);

      ({ popup, view } = await state({ port: await freePort(), token: server.token }));
      assert.deepEqual([view.state, view.title], ['bad', "tsumiba isn't running"]);
      // Trying anyway gives the same explanation, inline.
      await popup.evaluate(`document.getElementById('memo').value = 'nobody home'`);
      await popup.evaluate(`document.getElementById('save-memo').click()`);
      await waitFor(async () => /isn't running/.test(await text(popup, '#message')), { message: 'the inline error' });
      assert.equal(await popup.evaluate(`document.getElementById('message').dataset.kind`), 'error');
      await closeTab(popup);

      // Something else on the port.
      const other = http.createServer((req, res) => res.end('hello'));
      await new Promise((r) => other.listen(0, '127.0.0.1', r));
      try {
        ({ popup, view } = await state({ port: other.address().port, token: server.token }));
        assert.deepEqual([view.state, view.title], ['bad', "That isn't tsumiba"]);
        await closeTab(popup);
      } finally {
        other.close();
      }
    } finally {
      await closeTab(tab);
      await closeTab(admin);
    }
  });

  test('the background handlers work in real Chromium: page, selection, link and image', async () => {
    // import() is not allowed inside a service worker, so the module runs in an
    // extension page; the extension APIs and the network stack are the same.
    const page = await openTab(extensionUrl('options.html'));
    try {
      await pair(page, { port: server.port, token: server.token });
      const run = (info, tab = { id: 1, title: 'Fixture Page 日本語', url: fixtureUrl }) =>
        page.evaluate(`(async () => {
          const background = await import(chrome.runtime.getURL('background.js'));
          await background.onMenuClicked(${JSON.stringify({ pageUrl: fixtureUrl, ...info })}, ${JSON.stringify(tab)});
          return { text: await chrome.action.getBadgeText({}), colour: await chrome.action.getBadgeBackgroundColor({}) };
        })()`);

      let badge = await run({ menuItemId: 'tsumiba-page' });
      assert.equal(badge.text, '✓');
      assert.ok(server.list().notes.find((n) => n.body === pageNote({ title: 'Fixture Page 日本語', url: fixtureUrl })));

      badge = await run({ menuItemId: 'tsumiba-selection', selectionText: 'quoted\nwords' });
      assert.equal(badge.text, '✓');
      assert.ok(server.list().notes.find((n) => n.body.startsWith('> quoted\n> words\n\n[Fixture Page')));

      badge = await run({ menuItemId: 'tsumiba-link', linkUrl: 'https://target.example/x', linkText: 'the target' });
      assert.equal(badge.text, '✓');
      assert.ok(server.list().notes.find((n) => n.body.startsWith('# the target\nhttps://target.example/x\n\nFrom:')));

      // An image from another origin: fetched with the extension's host permission, uploaded as a file.
      badge = await run({ menuItemId: 'tsumiba-image', srcUrl: `${fixtureOrigin}/cat.png` });
      assert.equal(badge.text, '✓');
      const file = server.list().files.find((f) => f.name === 'cat.png' && f.source === fixtureUrl);
      assert.ok(file, JSON.stringify(server.list().files));
      assert.deepEqual(readFileSync(file.path), PNG);

      // Wrong token: a red mark, no data.
      await pair(page, { port: server.port, token: 'c'.repeat(64) });
      const before = server.list().notes.length;
      badge = await run({ menuItemId: 'tsumiba-page' });
      assert.equal(badge.text, '!');
      assert.equal(server.list().notes.length, before);

      assert.deepEqual(unexpected(page), []);
    } finally {
      await closeTab(page);
    }
  });

  test('an ordinary web page cannot use the API, even knowing the token', async () => {
    const tab = await openTab(fixtureUrl);
    try {
      const notesBefore = server.list().notes.length;
      const outcome = await tab.evaluate(`(async () => {
        const url = 'http://127.0.0.1:${server.port}/v1';
        const attempt = async (fn) => { try { return await fn(); } catch (e) { return 'blocked: ' + e.name; } };
        return {
          // A normal read: the browser refuses to hand the answer to this page.
          read: await attempt(async () => (await fetch(url + '/status', { headers: { Authorization: 'Bearer ${server.token}' } })).status),
          // The same, without a token.
          ping: await attempt(async () => (await fetch(url + '/ping')).status),
          // A "simple" cross-site POST is sent, but the app must refuse it.
          write: await attempt(async () => {
            const r = await fetch(url + '/notes', { method: 'POST', mode: 'no-cors', headers: { 'Content-Type': 'text/plain' }, body: JSON.stringify({ text: 'pwned' }) });
            return r.type;
          }),
        };
      })()`);
      assert.match(outcome.read, /blocked/);
      assert.match(outcome.ping, /blocked/);
      assert.equal(outcome.write, 'opaque', 'the request was sent but its answer is hidden from the page');
      await new Promise((r) => setTimeout(r, 300));
      const notes = server.list().notes;
      assert.equal(notes.length, notesBefore, 'nothing was written');
      assert.ok(!notes.some((n) => n.body.includes('pwned')));
    } finally {
      await closeTab(tab);
    }
  });
});
