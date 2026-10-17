// What the extension remembers: where tsumiba listens and the pairing token.
// Stored in chrome.storage.local, which never leaves this browser profile.

import { ext } from './ext.js';

export const DEFAULT_PORT = 47821;
export const MIN_PORT = 1024;
export const MAX_PORT = 65535;

export const DEFAULTS = Object.freeze({
  port: DEFAULT_PORT,
  token: '',
  // Set when the user said no to the "access all sites" prompt, so that
  // sending an image does not ask again every time.
  imagePermissionDenied: false,
});

/** A port number from a string or number, or `null` if it is not usable. */
export function parsePort(value) {
  const text = String(value ?? '').trim();
  if (!/^\d{1,5}$/.test(text)) return null;
  const port = Number(text);
  return port >= MIN_PORT && port <= MAX_PORT ? port : null;
}

/**
 * Tokens are 64 lowercase hex digits. Pasting often brings along whitespace or
 * a trailing newline, and the app only ever generates lowercase, so tidy both.
 */
export function normalizeToken(value) {
  return String(value ?? '').replace(/\s+/g, '').toLowerCase();
}

export function isValidToken(token) {
  return /^[0-9a-f]{64}$/.test(token);
}

export async function loadSettings() {
  const stored = await ext().storage.local.get(Object.keys(DEFAULTS));
  const port = parsePort(stored?.port);
  return {
    port: port ?? DEFAULTS.port,
    token: typeof stored?.token === 'string' ? stored.token : DEFAULTS.token,
    imagePermissionDenied: stored?.imagePermissionDenied === true,
  };
}

/** Save any subset of the settings. Throws on values that cannot be stored. */
export async function saveSettings(partial) {
  const update = {};
  if ('port' in partial) {
    const port = parsePort(partial.port);
    if (port === null) throw new RangeError(`port must be ${MIN_PORT}-${MAX_PORT}`);
    update.port = port;
  }
  if ('token' in partial) {
    const token = normalizeToken(partial.token);
    if (token !== '' && !isValidToken(token)) throw new TypeError('the token must be 64 hexadecimal digits');
    update.token = token;
  }
  if ('imagePermissionDenied' in partial) {
    update.imagePermissionDenied = partial.imagePermissionDenied === true;
  }
  await ext().storage.local.set(update);
  return loadSettings();
}
