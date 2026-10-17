// A small client for tsumiba's local HTTP API (documented in the app's docs
// under "HTTP API"). Everything goes to 127.0.0.1 and nowhere else.
//
// The client is built from explicit settings and an injectable `fetch`, so it
// can be tested without a browser.

export class TsumibaError extends Error {
  /**
   * @param {'not-running'|'not-tsumiba'|'not-paired'|'unauthorized'|'forbidden'|'too-large'|'bad-request'|'other'} code
   */
  constructor(code, message, { status, cause, limitBytes } = {}) {
    super(message, { cause });
    this.name = 'TsumibaError';
    this.code = code;
    this.status = status;
    // For `too-large` when the limit is known (the app tells us in /status).
    this.limitBytes = limitBytes;
  }
}

/**
 * Percent-encode a header value: everything outside [A-Za-z0-9._~-] becomes
 * %XX of its UTF-8 bytes, so the value is always printable ASCII. This is the
 * same rule as `encode_header` in tsumiba-core (the Rust tests share vectors).
 */
export function encodeHeader(value) {
  let out = '';
  for (const byte of new TextEncoder().encode(String(value))) {
    const c = String.fromCharCode(byte);
    out += /[A-Za-z0-9._~-]/.test(c) ? c : '%' + byte.toString(16).toUpperCase().padStart(2, '0');
  }
  return out;
}

export const MIB = 1024 * 1024;

/** The size in bytes of anything `fetch` accepts as a body. */
export function sizeOf(body) {
  if (body == null) return 0;
  if (typeof body.size === 'number') return body.size; // Blob, File
  if (typeof body.byteLength === 'number') return body.byteLength; // ArrayBuffer, typed arrays
  if (typeof body === 'string') return new TextEncoder().encode(body).length;
  return 0;
}

const DEFAULT_TIMEOUT_MS = 8000;
const UPLOAD_TIMEOUT_MS = 120000;

export function createClient({ port, token, fetch: fetchImpl = globalThis.fetch, timeoutMs } = {}) {
  const base = `http://127.0.0.1:${port}/v1`;

  async function request(method, path, { auth = true, headers = {}, body, timeout = timeoutMs ?? DEFAULT_TIMEOUT_MS } = {}) {
    const allHeaders = { ...headers };
    if (auth) allHeaders.Authorization = `Bearer ${token}`;

    let response;
    try {
      response = await fetchImpl(base + path, {
        method,
        headers: allHeaders,
        body,
        cache: 'no-store',
        credentials: 'omit',
        signal: AbortSignal.timeout(timeout),
      });
    } catch (cause) {
      if (cause?.name === 'TimeoutError') {
        throw new TsumibaError('other', 'tsumiba did not answer in time', { cause });
      }
      // fetch rejects with a TypeError when nothing is listening (or the
      // connection is refused or reset).
      throw new TsumibaError('not-running', `could not reach tsumiba on port ${port}`, { cause });
    }

    const text = await response.text().catch(() => '');
    let json = null;
    if (text) {
      try {
        json = JSON.parse(text);
      } catch {
        json = null;
      }
    }

    if (response.ok) return json;

    const message = json?.error ?? `HTTP ${response.status}`;
    switch (response.status) {
      case 401:
        throw new TsumibaError('unauthorized', message, { status: 401 });
      case 403:
        throw new TsumibaError('forbidden', message, { status: 403 });
      case 413:
        throw new TsumibaError('too-large', message, { status: 413 });
      case 400:
        throw new TsumibaError('bad-request', message, { status: 400 });
      default:
        throw new TsumibaError('other', message, { status: response.status });
    }
  }

  const client = {
    port,

    /** Is tsumiba there? Needs no token. Rejects with `not-tsumiba` for any other program. */
    async ping() {
      let info;
      try {
        info = await request('GET', '/ping', { auth: false });
      } catch (error) {
        // Another program answering on the port is not "tsumiba is off".
        if (error.code === 'other' || error.code === 'bad-request') {
          throw new TsumibaError('not-tsumiba', `something other than tsumiba answers on port ${port}`, {
            status: error.status,
          });
        }
        throw error;
      }
      if (!info || info.app !== 'tsumiba') {
        throw new TsumibaError('not-tsumiba', `something other than tsumiba answers on port ${port}`);
      }
      return info; // { app, version }
    },

    /** `{ notes, files, visible, max_upload_mb? }`. Needs the token. */
    status() {
      return request('GET', '/status');
    },

    /**
     * The largest upload the app accepts, in bytes, or `null` when it does not
     * say (older versions leave `max_upload_mb` out, and 0 means unknown).
     */
    async maxUploadBytes() {
      const status = await client.status();
      const mb = Number(status?.max_upload_mb);
      return Number.isFinite(mb) && mb >= 1 ? Math.floor(mb) * MIB : null;
    },

    createNote(text, { pinned = false } = {}) {
      return request('POST', '/notes', {
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text, pinned }),
      });
    },

    /**
     * Upload a file. `body` is a Blob/ArrayBuffer/Uint8Array; `name` is sanitised
     * by the app, `source` (usually the page URL) is shown in the file's tooltip.
     *
     * With `limitBytes`, a body over the limit is refused here as `too-large`
     * without being sent: when the app rejects an upload early it closes the
     * connection while the body is still travelling, and on some systems the
     * client then sees a network error instead of the 413.
     */
    uploadFile({ name, source, body, contentType }, { limitBytes } = {}) {
      if (limitBytes && sizeOf(body) > limitBytes) {
        return Promise.reject(
          new TsumibaError('too-large', `the file is larger than the ${Math.round(limitBytes / MIB)} MiB tsumiba accepts`, {
            limitBytes,
          }),
        );
      }
      const headers = {
        'Content-Type': contentType || 'application/octet-stream',
        'X-Tsumiba-Name': encodeHeader(name || 'file'),
      };
      if (source) headers['X-Tsumiba-Source'] = encodeHeader(source);
      return request('POST', '/files', { headers, body, timeout: UPLOAD_TIMEOUT_MS });
    },

    /** Look up the app's upload limit, then upload (see `uploadFile`). */
    async uploadFileChecked(file) {
      const limitBytes = await client.maxUploadBytes();
      return client.uploadFile(file, { limitBytes });
    },

    /** Ask the app to show, hide or toggle the shelf. */
    window(action) {
      return request('POST', '/window', {
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action }),
      });
    },
  };
  return client;
}

/**
 * Work out, precisely, why things do or do not work. Returns
 * `{ state, version?, notes?, files? }` where state is one of
 * `not-paired`, `not-running`, `not-tsumiba`, `unauthorized`, `forbidden`,
 * `error` or `ok`.
 */
export async function checkConnection({ port, token, fetch: fetchImpl }) {
  const client = createClient({ port, token, fetch: fetchImpl, timeoutMs: 3000 });
  let info;
  try {
    info = await client.ping();
  } catch (error) {
    return { state: error.code === 'other' ? 'error' : error.code, error };
  }
  if (!token) return { state: 'not-paired', version: info.version };
  try {
    const status = await client.status();
    return { state: 'ok', version: info.version, notes: status.notes, files: status.files };
  } catch (error) {
    return { state: error.code === 'other' ? 'error' : error.code, version: info.version, error };
  }
}
