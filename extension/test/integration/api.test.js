// lib/api.js against the real tsumiba (headless `serve`), plus the CORS
// behaviour a browser relies on.

import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import http from 'node:http';
import { after, before, describe, test } from 'node:test';

import { TsumibaError, checkConnection, createClient, encodeHeader } from '../../src/lib/api.js';
import { needsBinary, startServer } from '../helpers/server.js';

const EXTENSION_ORIGIN = 'chrome-extension://abcdefghijklmnopabcdefghijklmnop';

/** A raw request, so the test controls Origin and Host exactly like a browser would. */
function raw(port, { method = 'GET', path, headers = {} }) {
  return new Promise((resolve, reject) => {
    const request = http.request({ host: '127.0.0.1', port, method, path, headers }, (response) => {
      const chunks = [];
      response.on('data', (c) => chunks.push(c));
      response.on('end', () =>
        resolve({ status: response.statusCode, headers: response.headers, body: Buffer.concat(chunks).toString('utf8') }),
      );
    });
    request.on('error', reject);
    request.end();
  });
}

describe('lib/api.js against the real app', needsBinary, () => {
  let server;
  let client;

  before(async () => {
    server = await startServer({ config: '[storage]\nmax_upload_mb = 1\n' });
    client = createClient({ port: server.port, token: server.token });
  });
  after(async () => {
    await server?.cleanup();
  });

  test('ping works without a token and reports the version', async () => {
    const info = await createClient({ port: server.port, token: '' }).ping();
    assert.equal(info.app, 'tsumiba');
    assert.match(info.version, /^\d+\.\d+\.\d+/);
  });

  test('status needs the right token', async () => {
    const ok = await client.status();
    assert.equal(typeof ok.notes, 'number');
    assert.equal(typeof ok.files, 'number');

    for (const token of ['wrong', '', server.token.slice(1), server.token.toUpperCase()]) {
      const error = await createClient({ port: server.port, token }).status().catch((e) => e);
      assert.ok(error instanceof TsumibaError, String(error));
      assert.equal(error.code, 'unauthorized', `token ${JSON.stringify(token)}`);
      assert.equal(error.status, 401);
    }
  });

  test('checkConnection tells the situations apart', async () => {
    const good = await checkConnection({ port: server.port, token: server.token });
    assert.equal(good.state, 'ok');
    assert.match(good.version, /^\d+\.\d+\.\d+/);

    assert.equal((await checkConnection({ port: server.port, token: 'x'.repeat(64) })).state, 'unauthorized');
    assert.equal((await checkConnection({ port: server.port, token: '' })).state, 'not-paired');
  });

  test('a note created through the client is visible to the command line', async () => {
    const text = '# 日本語のメモ\nfrom the extension test\n';
    const note = await client.createNote(text);
    assert.equal(note.title, '日本語のメモ');
    assert.equal(note.body, text);
    assert.equal(note.pinned, false);

    const listed = server.list().notes.find((n) => n.id === note.id);
    assert.ok(listed, 'the CLI lists it');
    assert.equal(listed.body, text);
  });

  test('a note can be created pinned', async () => {
    const note = await client.createNote('pinned from the extension', { pinned: true });
    assert.equal(note.pinned, true);
    assert.equal(server.list().notes.find((n) => n.id === note.id).pinned, true);
  });

  test('every note format the extension writes survives the round trip byte for byte', async () => {
    const { pageNote, selectionNote, linkNote, imageNote, memoNote } = await import('../../src/lib/format.js');
    const bodies = [
      pageNote({ title: 'ページ【テスト】', url: 'https://example.com/a?b=c&d=日本' }),
      selectionNote({ selection: 'line one\r\n\r\nline 日本語', title: 'T [x]', url: 'https://example.com/(a)' }),
      linkNote({ href: 'https://t.example/x', text: 'リンク', pageTitle: 'P', pageUrl: 'https://p.example/' }),
      imageNote({ src: 'https://cdn.example/猫.png', fileName: '猫.png', pageUrl: 'https://p.example/' }),
      memoNote('  indented first line\nsecond\n\n'),
    ];
    for (const body of bodies) {
      const note = await client.createNote(body);
      assert.equal(note.body, body);
      assert.equal(readFileSync(`${server.home}/data/notes/${note.id}.md`, 'utf8'), body);
    }
  });

  test('an uploaded file keeps its Japanese name, its bytes and its source', async () => {
    const bytes = randomBytes(5000);
    const source = 'https://example.com/ギャラリー?id=7&x=y#frag';
    const file = await client.uploadFile({
      name: 'スクリーンショット (1).png',
      source,
      body: new Blob([bytes], { type: 'image/png' }),
      contentType: 'image/png',
    });
    assert.equal(file.name, 'スクリーンショット (1).png');
    assert.equal(file.size, 5000);
    assert.equal(file.source, source);
    assert.equal(file.link, false);
    assert.deepEqual(readFileSync(file.path), bytes);

    const listed = server.list().files.find((f) => f.id === file.id);
    assert.equal(listed.name, 'スクリーンショット (1).png');
    assert.equal(listed.source, source);
    assert.equal(listed.size, 5000);
  });

  test('every header-encoding vector from the Rust tests survives as a file name', async () => {
    // Hostile names get sanitised by the app; what matters here is that the
    // percent-decoding on the server side undoes encodeHeader exactly.
    const cases = [
      ['plain.txt', 'plain.txt'],
      ['日本語 メモ (1).png', '日本語 メモ (1).png'],
      ['100%.txt', '100%.txt'],
      ['%zz.txt', '%zz.txt'],
      ['😀.png', '😀.png'],
    ];
    for (const [name, expected] of cases) {
      const file = await client.uploadFile({ name, body: new Blob(['x']), contentType: 'application/octet-stream' });
      assert.equal(file.name, expected, `${name} -> ${encodeHeader(name)}`);
    }
  });

  test('the app tidies hostile names the extension might forward', async () => {
    const file = await client.uploadFile({ name: '../../etc/passwd', body: new Blob(['x']) });
    assert.equal(file.name, 'passwd');
    assert.ok(file.path.startsWith(server.home), `${file.path} stays inside the data directory`);
  });

  test('a name without an extension gets one from the content type', async () => {
    const file = await client.uploadFile({ name: 'render', body: new Blob(['x']), contentType: 'image/webp' });
    assert.equal(file.name, 'render.webp');
  });

  test('an upload over the limit is reported as too-large and leaves nothing behind', async () => {
    const before = server.list().files.length;
    const over = new Uint8Array(1024 * 1024 + 1);
    const error = await client
      .uploadFile({ name: 'big.bin', body: new Blob([over]), contentType: 'application/octet-stream' })
      .catch((e) => e);
    assert.ok(error instanceof TsumibaError, String(error?.cause ?? error));
    assert.equal(error.code, 'too-large');
    assert.equal(error.status, 413);
    assert.equal(server.list().files.length, before);

    // Exactly at the limit is fine.
    const exact = await client.uploadFile({ name: 'exact.bin', body: new Blob([new Uint8Array(1024 * 1024)]) });
    assert.equal(exact.size, 1024 * 1024);
  });

  test('the app announces its upload limit in /status (or, if it is an older build, says nothing)', async () => {
    const limit = await client.maxUploadBytes();
    // This server is configured with max_upload_mb = 1.
    assert.ok(limit === null || limit === 1024 * 1024, `unexpected limit ${limit}`);
    if (limit === null) console.log('# note: this tsumiba build does not report max_upload_mb yet');
  });

  test('uploadFileChecked: an oversized file is refused whether or not the app announces a limit', async () => {
    const before = server.list().files.length;
    const error = await client
      .uploadFileChecked({ name: 'big.bin', body: new Blob([new Uint8Array(3 * 1024 * 1024)]) })
      .catch((e) => e);
    assert.equal(error.code, 'too-large', String(error.cause ?? error.message));
    assert.equal(server.list().files.length, before);
    const ok = await client.uploadFileChecked({ name: 'small.bin', body: new Blob([new Uint8Array(1000)]) });
    assert.equal(ok.size, 1000);
  });

  test('a much larger upload is still reported as too-large, not as "not running"', async () => {
    const error = await client
      .uploadFile({ name: 'huge.bin', body: new Blob([new Uint8Array(8 * 1024 * 1024)]) })
      .catch((e) => e);
    assert.equal(error.code, 'too-large', String(error.cause ?? error.message));
  });

  test('window requests are accepted', async () => {
    assert.equal(await client.window('show'), null);
    assert.equal(await client.window('hide'), null);
    assert.equal(await client.window('toggle'), null);
    const error = await client.window('explode').catch((e) => e);
    assert.equal(error.code, 'bad-request');
  });

  // ---- what a browser does before and during those requests ------------------

  test('the preflight an extension triggers succeeds and allows exactly the headers the client sends', async () => {
    // Record every header the client really sends.
    const sent = new Set();
    const methods = new Set();
    const recording = createClient({
      port: server.port,
      token: server.token,
      fetch: (url, init) => {
        for (const name of Object.keys(init.headers ?? {})) sent.add(name.toLowerCase());
        methods.add(init.method);
        return fetch(url, init);
      },
    });
    await recording.status();
    await recording.createNote('x');
    await recording.uploadFile({ name: 'a.txt', source: 'https://s.example/', body: new Blob(['x']), contentType: 'text/plain' });
    await recording.window('toggle');

    const preflight = await raw(server.port, {
      method: 'OPTIONS',
      path: '/v1/files',
      headers: {
        Host: `127.0.0.1:${server.port}`,
        Origin: EXTENSION_ORIGIN,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': [...sent].sort().join(','),
      },
    });
    assert.equal(preflight.status, 204);
    assert.equal(preflight.headers['access-control-allow-origin'], EXTENSION_ORIGIN);
    const allowedHeaders = preflight.headers['access-control-allow-headers'].split(',').map((h) => h.trim().toLowerCase());
    for (const header of sent) assert.ok(allowedHeaders.includes(header), `the client sends "${header}" but the preflight does not allow it (${allowedHeaders})`);
    const allowedMethods = preflight.headers['access-control-allow-methods'].split(',').map((m) => m.trim());
    for (const method of methods) assert.ok(allowedMethods.includes(method), `${method} is not allowed`);
  });

  test('the preflight needs no token (browsers do not send one on it)', async () => {
    const response = await raw(server.port, {
      method: 'OPTIONS',
      path: '/v1/notes',
      headers: { Host: `127.0.0.1:${server.port}`, Origin: EXTENSION_ORIGIN, 'Access-Control-Request-Method': 'POST' },
    });
    assert.equal(response.status, 204);
  });

  test('real responses echo an extension origin so the browser lets the page read them', async () => {
    for (const origin of [EXTENSION_ORIGIN, 'moz-extension://5b0b2f8e-0000-4000-8000-000000000000']) {
      const response = await raw(server.port, {
        path: '/v1/status',
        headers: { Host: `127.0.0.1:${server.port}`, Origin: origin, Authorization: `Bearer ${server.token}` },
      });
      assert.equal(response.status, 200);
      assert.equal(response.headers['access-control-allow-origin'], origin);
      assert.equal(response.headers.vary, 'Origin');
    }
  });

  test('a web page is refused, with or without the token', async () => {
    for (const origin of ['https://evil.example', 'http://localhost:3000', 'null']) {
      const withToken = await raw(server.port, {
        path: '/v1/notes',
        headers: { Host: `127.0.0.1:${server.port}`, Origin: origin, Authorization: `Bearer ${server.token}` },
      });
      assert.equal(withToken.status, 403, origin);
      assert.equal(withToken.headers['access-control-allow-origin'], undefined);

      const preflight = await raw(server.port, {
        method: 'OPTIONS',
        path: '/v1/notes',
        headers: { Host: `127.0.0.1:${server.port}`, Origin: origin, 'Access-Control-Request-Method': 'POST' },
      });
      assert.equal(preflight.status, 403, `preflight from ${origin}`);
    }
  });

  test('a rebinding attack (foreign Host) is refused', async () => {
    const response = await raw(server.port, {
      path: '/v1/status',
      headers: { Host: 'evil.example:80', Authorization: `Bearer ${server.token}` },
    });
    assert.equal(response.status, 403);
  });

  test('requests from the client never carry cookies', async () => {
    let seen;
    const spying = createClient({
      port: server.port,
      token: server.token,
      fetch: (url, init) => {
        seen = init;
        return fetch(url, init);
      },
    });
    await spying.status();
    assert.equal(seen.credentials, 'omit');
  });
});

describe('lib/api.js when the app is not there', needsBinary, () => {
  test('a stopped app is "not-running", and so is a port nobody opened', async () => {
    const server = await startServer();
    const { port, token } = server;
    const client = createClient({ port, token });
    assert.equal((await client.ping()).app, 'tsumiba');

    await server.stop();
    for (const call of [() => client.ping(), () => client.status(), () => client.createNote('x'), () => client.uploadFile({ name: 'a', body: new Blob(['x']) })]) {
      const error = await call().catch((e) => e);
      assert.ok(error instanceof TsumibaError);
      assert.equal(error.code, 'not-running');
    }
    assert.equal((await checkConnection({ port, token })).state, 'not-running');
    await server.cleanup();
  });

  test('another program on the port is "not-tsumiba"', async () => {
    for (const respond of [
      (res) => {
        res.statusCode = 404;
        res.end('Not Found');
      },
      (res) => {
        res.setHeader('Content-Type', 'application/json');
        res.end(JSON.stringify({ app: 'definitely-not-tsumiba', version: '9' }));
      },
      (res) => res.end('<html>hello</html>'),
    ]) {
      const other = http.createServer((req, res) => respond(res));
      await new Promise((r) => other.listen(0, '127.0.0.1', r));
      try {
        const { port } = other.address();
        const error = await createClient({ port, token: 't' }).ping().catch((e) => e);
        assert.equal(error.code, 'not-tsumiba');
        assert.equal((await checkConnection({ port, token: 't' })).state, 'not-tsumiba');
      } finally {
        await new Promise((r) => other.close(r));
      }
    }
  });
});
