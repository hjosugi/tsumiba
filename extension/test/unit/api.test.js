import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MIB, TsumibaError, checkConnection, createClient, encodeHeader, sizeOf } from '../../src/lib/api.js';

// ---- header encoding: the same vectors as `encode_header` in tsumiba-core ----

test('header values are percent-encoded UTF-8, exactly like the app does', () => {
  const vectors = [
    ['plain.txt', 'plain.txt'],
    ['日本語 メモ (1).png', '%E6%97%A5%E6%9C%AC%E8%AA%9E%20%E3%83%A1%E3%83%A2%20%281%29.png'],
    ['a/b\\c?d=e&f#g', 'a%2Fb%5Cc%3Fd%3De%26f%23g'],
    ['100%', '100%25'],
    ['%zz', '%25zz'],
    ['', ''],
  ];
  for (const [input, expected] of vectors) {
    assert.equal(encodeHeader(input), expected, JSON.stringify(input));
    assert.equal(decodeURIComponent(encodeHeader(input)), input, `round trip of ${JSON.stringify(input)}`);
  }
});

test('encoded headers are printable ASCII whatever goes in', () => {
  const encoded = encodeHeader('日本語\n"quoted"\t\u0000 😀');
  assert.match(encoded, /^[\x21-\x7e]*$/);
  assert.equal(decodeURIComponent(encoded), '日本語\n"quoted"\t\u0000 😀');
});

test('unreserved characters are left alone', () => {
  assert.equal(encodeHeader('AZaz09._~-'), 'AZaz09._~-');
});

// ---- the client against a scripted fetch ------------------------------------

function scripted(handler) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, init });
    return handler(url, init);
  };
  return { fetch, calls };
}

const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

test('requests go to loopback only, with the token, without cookies', async () => {
  const { fetch, calls } = scripted(() => json(200, { notes: 1, files: 2, visible: false }));
  const client = createClient({ port: 51234, token: 'abc', fetch });
  assert.deepEqual(await client.status(), { notes: 1, files: 2, visible: false });
  const { url, init } = calls[0];
  assert.equal(url, 'http://127.0.0.1:51234/v1/status');
  assert.equal(init.headers.Authorization, 'Bearer abc');
  assert.equal(init.credentials, 'omit');
  assert.equal(init.cache, 'no-store');
  assert.ok(init.signal instanceof AbortSignal, 'every request has a timeout');
});

test('ping needs no token', async () => {
  const { fetch, calls } = scripted(() => json(200, { app: 'tsumiba', version: '0.1.0' }));
  const info = await createClient({ port: 1, token: 'abc', fetch }).ping();
  assert.deepEqual(info, { app: 'tsumiba', version: '0.1.0' });
  assert.equal(calls[0].init.headers.Authorization, undefined);
});

test('creating a note posts JSON', async () => {
  const { fetch, calls } = scripted(() => json(201, { id: 'x' }));
  await createClientFor(fetch).createNote('hello 日本語', { pinned: true });
  const { url, init } = calls[0];
  assert.equal(url, 'http://127.0.0.1:47821/v1/notes');
  assert.equal(init.method, 'POST');
  assert.equal(init.headers['Content-Type'], 'application/json');
  assert.deepEqual(JSON.parse(init.body), { text: 'hello 日本語', pinned: true });
});

function createClientFor(fetch) {
  return createClient({ port: 47821, token: 't', fetch });
}

test('uploading sends the name and source encoded, and the content type', async () => {
  const { fetch, calls } = scripted(() => json(201, { id: 'x' }));
  const body = new Blob(['bytes'], { type: 'image/png' });
  await createClientFor(fetch).uploadFile({
    name: '猫 (1).png',
    source: 'https://example.com/ねこ?a=b',
    body,
    contentType: 'image/png',
  });
  const { url, init } = calls[0];
  assert.equal(url, 'http://127.0.0.1:47821/v1/files');
  assert.equal(init.headers['Content-Type'], 'image/png');
  assert.equal(init.headers['X-Tsumiba-Name'], encodeHeader('猫 (1).png'));
  assert.equal(init.headers['X-Tsumiba-Source'], encodeHeader('https://example.com/ねこ?a=b'));
  assert.equal(init.body, body);
});

test('an upload without a source or type omits the source and defaults the type', async () => {
  const { fetch, calls } = scripted(() => json(201, {}));
  await createClientFor(fetch).uploadFile({ name: 'a.bin', body: new Uint8Array([1]) });
  assert.equal(calls[0].init.headers['X-Tsumiba-Source'], undefined);
  assert.equal(calls[0].init.headers['Content-Type'], 'application/octet-stream');
});

test('window actions are posted as JSON and a 204 is fine', async () => {
  const { fetch, calls } = scripted(() => new Response(null, { status: 204 }));
  assert.equal(await createClientFor(fetch).window('show'), null);
  assert.deepEqual(JSON.parse(calls[0].init.body), { action: 'show' });
});

// ---- error mapping ------------------------------------------------------------

async function codeOf(promise) {
  try {
    await promise;
  } catch (error) {
    assert.ok(error instanceof TsumibaError, `${error}`);
    return error.code;
  }
  return 'resolved';
}

test('HTTP statuses map to friendly error codes', async () => {
  const cases = [
    [401, 'unauthorized'],
    [403, 'forbidden'],
    [413, 'too-large'],
    [400, 'bad-request'],
    [404, 'other'],
    [500, 'other'],
  ];
  for (const [status, code] of cases) {
    const { fetch } = scripted(() => json(status, { error: 'because' }));
    const error = await createClientFor(fetch).status().catch((e) => e);
    assert.equal(error.code, code, `status ${status}`);
    assert.equal(error.status, status);
    assert.equal(error.message, 'because', 'the server\'s message is kept');
  }
});

test('an error body that is not JSON still gives a usable error', async () => {
  const { fetch } = scripted(() => new Response('<html>nope</html>', { status: 502 }));
  const error = await createClientFor(fetch).status().catch((e) => e);
  assert.equal(error.code, 'other');
  assert.equal(error.message, 'HTTP 502');
});

test('nothing listening means not-running', async () => {
  const { fetch } = scripted(() => {
    throw new TypeError('fetch failed');
  });
  assert.equal(await codeOf(createClientFor(fetch).status()), 'not-running');
  assert.equal(await codeOf(createClientFor(fetch).ping()), 'not-running');
  assert.equal(await codeOf(createClientFor(fetch).createNote('x')), 'not-running');
});

test('a timeout is reported as such, not as "not running"', async () => {
  const { fetch } = scripted(() => {
    throw new DOMException('The operation timed out.', 'TimeoutError');
  });
  const error = await createClientFor(fetch).status().catch((e) => e);
  assert.equal(error.code, 'other');
  assert.match(error.message, /in time/);
});

test('another program on the port is not-tsumiba', async () => {
  const cases = [
    () => new Response('Not Found', { status: 404 }),
    () => json(200, { app: 'something-else', version: '1' }),
    () => new Response('<html></html>', { status: 200 }),
    () => json(200, null),
  ];
  for (const respond of cases) {
    const { fetch } = scripted(respond);
    assert.equal(await codeOf(createClientFor(fetch).ping()), 'not-tsumiba');
  }
});

// ---- the connection check -------------------------------------------------------

function route(table) {
  return scripted((url) => {
    const path = new URL(url).pathname.replace('/v1', '');
    const entry = table[path];
    if (entry instanceof Error) throw entry;
    return entry();
  });
}

const OK_PING = () => json(200, { app: 'tsumiba', version: '0.1.0' });

test('checkConnection: connected', async () => {
  const { fetch } = route({ '/ping': OK_PING, '/status': () => json(200, { notes: 3, files: 7, visible: true }) });
  assert.deepEqual(await checkConnection({ port: 1, token: 'tok', fetch }), {
    state: 'ok',
    version: '0.1.0',
    notes: 3,
    files: 7,
  });
});

test('checkConnection: not running', async () => {
  const { fetch } = route({ '/ping': new TypeError('fetch failed') });
  assert.equal((await checkConnection({ port: 1, token: 'tok', fetch })).state, 'not-running');
});

test('checkConnection: not paired skips the authenticated call', async () => {
  const { fetch, calls } = route({ '/ping': OK_PING });
  const result = await checkConnection({ port: 1, token: '', fetch });
  assert.equal(result.state, 'not-paired');
  assert.equal(result.version, '0.1.0');
  assert.equal(calls.length, 1, 'only the open ping was made');
});

test('checkConnection: wrong token keeps the version for the message', async () => {
  const { fetch } = route({ '/ping': OK_PING, '/status': () => json(401, { error: 'missing or wrong token' }) });
  const result = await checkConnection({ port: 1, token: 'bad', fetch });
  assert.equal(result.state, 'unauthorized');
  assert.equal(result.version, '0.1.0');
});

test('checkConnection: something else on the port', async () => {
  const { fetch } = route({ '/ping': () => new Response('nope', { status: 404 }) });
  assert.equal((await checkConnection({ port: 1, token: 't', fetch })).state, 'not-tsumiba');
});

test('checkConnection: refused host header, and unexpected failures', async () => {
  let { fetch } = route({ '/ping': OK_PING, '/status': () => json(403, { error: 'unexpected Host header' }) });
  assert.equal((await checkConnection({ port: 1, token: 't', fetch })).state, 'forbidden');
  ({ fetch } = route({ '/ping': OK_PING, '/status': () => json(500, { error: 'boom' }) }));
  const result = await checkConnection({ port: 1, token: 't', fetch });
  assert.equal(result.state, 'error');
  assert.equal(result.error.message, 'boom');
});

// ---- the upload limit (GET /v1/status -> max_upload_mb) --------------------------

test('maxUploadBytes reads max_upload_mb from the status', async () => {
  const { fetch } = scripted(() => json(200, { notes: 0, files: 0, visible: false, max_upload_mb: 512 }));
  assert.equal(await createClientFor(fetch).maxUploadBytes(), 512 * MIB);
});

test('an older app that does not say gives null, as does 0 or nonsense', async () => {
  for (const body of [
    { notes: 0, files: 0, visible: false },
    { max_upload_mb: 0 },
    { max_upload_mb: null },
    { max_upload_mb: 'lots' },
    { max_upload_mb: -5 },
    { max_upload_mb: 0.5 },
  ]) {
    const { fetch } = scripted(() => json(200, body));
    assert.equal(await createClientFor(fetch).maxUploadBytes(), null, JSON.stringify(body));
  }
});

test('maxUploadBytes passes on a failing status (wrong token, app stopped)', async () => {
  let { fetch } = scripted(() => json(401, { error: 'no' }));
  assert.equal(await codeOf(createClientFor(fetch).maxUploadBytes()), 'unauthorized');
  ({ fetch } = scripted(() => {
    throw new TypeError('fetch failed');
  }));
  assert.equal(await codeOf(createClientFor(fetch).maxUploadBytes()), 'not-running');
});

test('a body over the limit is refused without sending anything', async () => {
  const { fetch, calls } = scripted(() => json(201, {}));
  const client = createClientFor(fetch);
  const error = await client
    .uploadFile({ name: 'big.bin', body: new Blob([new Uint8Array(MIB + 1)]) }, { limitBytes: MIB })
    .catch((e) => e);
  assert.equal(error.code, 'too-large');
  assert.equal(error.limitBytes, MIB);
  assert.equal(calls.length, 0, 'no request was made');
});

test('a body exactly at the limit is sent; no limit means no check', async () => {
  const { fetch, calls } = scripted(() => json(201, {}));
  const client = createClientFor(fetch);
  await client.uploadFile({ name: 'a', body: new Blob([new Uint8Array(MIB)]) }, { limitBytes: MIB });
  await client.uploadFile({ name: 'b', body: new Blob([new Uint8Array(MIB + 1)]) }, { limitBytes: null });
  await client.uploadFile({ name: 'c', body: new Blob([new Uint8Array(MIB + 1)]) });
  assert.equal(calls.length, 3);
});

test('uploadFileChecked looks up the limit first, then uploads', async () => {
  const { fetch, calls } = route({
    '/status': () => json(200, { notes: 0, files: 0, visible: false, max_upload_mb: 1 }),
    '/files': () => json(201, { id: 'f' }),
  });
  const client = createClientFor(fetch);
  await client.uploadFileChecked({ name: 'ok.bin', body: new Blob([new Uint8Array(100)]) });
  assert.deepEqual(calls.map((c) => new URL(c.url).pathname), ['/v1/status', '/v1/files']);

  calls.length = 0;
  const error = await client.uploadFileChecked({ name: 'big.bin', body: new Blob([new Uint8Array(2 * MIB)]) }).catch((e) => e);
  assert.equal(error.code, 'too-large');
  assert.deepEqual(calls.map((c) => new URL(c.url).pathname), ['/v1/status'], 'only the status was asked; the body never left');
});

test('uploadFileChecked still uploads when the app does not announce a limit', async () => {
  const { fetch, calls } = route({
    '/status': () => json(200, { notes: 0, files: 0, visible: false }),
    '/files': () => json(201, { id: 'f' }),
  });
  await createClientFor(fetch).uploadFileChecked({ name: 'big.bin', body: new Blob([new Uint8Array(5 * MIB)]) });
  assert.deepEqual(calls.map((c) => new URL(c.url).pathname), ['/v1/status', '/v1/files']);
});

test('sizeOf handles every body type the extension uses', () => {
  assert.equal(sizeOf(new Blob(['abc'])), 3);
  assert.equal(sizeOf(new Uint8Array(7)), 7);
  assert.equal(sizeOf(new ArrayBuffer(9)), 9);
  assert.equal(sizeOf('日本'), 6);
  assert.equal(sizeOf(null), 0);
  assert.equal(sizeOf(undefined), 0);
});
