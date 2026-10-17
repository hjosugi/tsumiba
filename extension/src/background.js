// The background script: Chrome runs it as a service worker, Firefox as an
// event page. It owns the context menus and the keyboard shortcut, and gives
// feedback through the toolbar badge.
//
// Everything here is event-driven: nothing is kept in memory that the browser
// could not rebuild after unloading the script.

import { TsumibaError, createClient } from './lib/api.js';
import { ext } from './lib/ext.js';
import { imageNote, linkNote, pageNote, selectionNote } from './lib/format.js';
import { errorMessage, pickLang, t } from './lib/i18n.js';
import { imageFileName, isDataUrl, looksLikeImage } from './lib/images.js';
import { loadSettings, saveSettings } from './lib/settings.js';

/** How long the ✓ / ! badge stays up. */
export const BADGE_MS = 2500;

const ALL_SITES = { origins: ['<all_urls>'] };

const MENUS = [
  { id: 'tsumiba-selection', contexts: ['selection'], title: 'menuSelection' },
  { id: 'tsumiba-link', contexts: ['link'], title: 'menuLink' },
  { id: 'tsumiba-image', contexts: ['image'], title: 'menuImage' },
  { id: 'tsumiba-page', contexts: ['page'], title: 'menuPage' },
];

function lang() {
  return pickLang(globalThis.navigator?.language);
}

// ---- context menus ---------------------------------------------------------

// Menus are rebuilt from scratch every time, one rebuild at a time: the install
// event, browser start-up and the defensive call at the bottom of this file can
// all land together, and Chrome refuses duplicate ids.
let menuQueue = Promise.resolve();

export function ensureMenus() {
  menuQueue = menuQueue.then(rebuildMenus, rebuildMenus);
  return menuQueue;
}

async function rebuildMenus() {
  const api = ext();
  const menus = api.contextMenus ?? api.menus;
  if (!menus) return;
  try {
    // Promise form: Chrome (MV3) and Firefox both return one.
    await menus.removeAll();
    for (const item of MENUS) {
      await new Promise((resolve) => {
        menus.create({ id: item.id, title: t(lang(), item.title), contexts: item.contexts }, () => {
          const error = api.runtime.lastError;
          if (error) console.warn(`tsumiba: could not create menu ${item.id}: ${error.message}`);
          resolve();
        });
      });
    }
  } catch (error) {
    console.warn('tsumiba: could not build the context menus', error);
  }
}

// ---- feedback --------------------------------------------------------------

let clearTimer = null;

async function showBadge(ok, message) {
  const action = ext().action;
  if (!action) return;
  await Promise.allSettled([
    action.setBadgeText({ text: ok ? '✓' : '!' }),
    action.setBadgeBackgroundColor({ color: ok ? '#2e9d57' : '#d6453d' }),
    action.setBadgeTextColor?.({ color: '#ffffff' }),
    action.setTitle({ title: `tsumiba — ${message}` }),
  ]);
  clearTimeout(clearTimer);
  clearTimer = setTimeout(clearBadge, BADGE_MS);
}

async function clearBadge() {
  const action = ext().action;
  if (!action) return;
  await Promise.allSettled([action.setBadgeText({ text: '' }), action.setTitle({ title: 'tsumiba' })]);
}

// ---- sending ---------------------------------------------------------------

async function connect() {
  const settings = await loadSettings();
  if (!settings.token) {
    throw new TsumibaError('not-paired', 'no token has been set');
  }
  return { settings, api: createClient(settings) };
}

/** Run `work`, then tell the user how it went. Resolves either way. */
async function run(work) {
  try {
    const messageKey = await work();
    await showBadge(true, t(lang(), messageKey ?? 'sentOk'));
  } catch (error) {
    const known = error instanceof TsumibaError;
    if (!known) console.error('tsumiba:', error);
    if (error?.code === 'not-paired') ext().runtime.openOptionsPage?.();
    await showBadge(false, errorMessage(lang(), known ? error : new TsumibaError('other', String(error?.message ?? error))));
  }
}

function sameOrigin(a, b) {
  try {
    return new URL(a).origin === new URL(b).origin;
  } catch {
    return false;
  }
}

/**
 * Get an image's bytes. Tries a plain fetch first; if that fails (usually the
 * site does not allow it), asks once for access to all sites and retries.
 * Returns `null` when the image cannot be had, so the caller can save its
 * address instead.
 */
async function fetchImage(src, pageUrl, settings) {
  const attempt = async () => {
    const response = await fetch(src, {
      credentials: sameOrigin(src, pageUrl) ? 'include' : 'omit',
    });
    if (!response.ok) return null; // hotlink protection, 404 …: not a permissions problem
    const blob = await response.blob();
    return looksLikeImage(blob.type) ? blob : null;
  };

  try {
    return await attempt();
  } catch {
    // Blocked or unreachable. Fall through to the permission path.
  }

  // A blob: URL belongs to the page that made it; no permission helps.
  if (/^blob:/i.test(src)) return null;

  const permissions = ext().permissions;
  if (!permissions) return null;
  try {
    if (await permissions.contains(ALL_SITES)) return null; // already allowed, still failed
    if (settings.imagePermissionDenied) return null; // said no before: do not nag
    // This only works inside a user gesture, which a menu click is.
    const granted = await permissions.request(ALL_SITES);
    await saveSettings({ imagePermissionDenied: !granted });
    if (!granted) return null;
    return await attempt();
  } catch {
    return null;
  }
}

async function sendImage(info, tab) {
  const { settings, api } = await connect();
  const src = info.srcUrl;
  if (!src) throw new TsumibaError('bad-request', 'no image address');
  const pageUrl = info.pageUrl ?? tab?.url;
  const pageTitle = tab?.title;

  const blob = await fetchImage(src, pageUrl, settings);
  if (blob) {
    await api.uploadFileChecked({
      name: imageFileName({ src, contentType: blob.type }),
      source: pageUrl,
      body: blob,
      contentType: blob.type,
    });
    return 'sentImageFile';
  }
  if (isDataUrl(src)) throw new TsumibaError('bad-request', 'the embedded image could not be read');
  await api.createNote(
    imageNote({ src, fileName: imageFileName({ src }), pageTitle, pageUrl }),
  );
  return 'sentImageLink';
}

async function sendPage(tab) {
  const { api } = await connect();
  if (!tab?.url) throw new TsumibaError('bad-request', t(lang(), 'errEmpty'));
  await api.createNote(pageNote({ title: tab.title, url: tab.url }));
}

async function sendSelection(info, tab) {
  const { api } = await connect();
  if (!info.selectionText?.trim()) throw new TsumibaError('bad-request', t(lang(), 'errEmpty'));
  await api.createNote(
    selectionNote({
      selection: info.selectionText,
      title: tab?.title,
      url: info.pageUrl ?? tab?.url,
    }),
  );
}

async function sendLink(info, tab) {
  const { api } = await connect();
  if (!info.linkUrl) throw new TsumibaError('bad-request', t(lang(), 'errEmpty'));
  await api.createNote(
    linkNote({
      href: info.linkUrl,
      // Firefox says what the link was called; Chrome only if it was selected.
      text: info.linkText || info.selectionText || '',
      pageTitle: tab?.title,
      pageUrl: info.pageUrl ?? tab?.url,
    }),
  );
}

/** Exported so that the browser smoke test can drive the real handlers. */
export function onMenuClicked(info, tab) {
  switch (info.menuItemId) {
    case 'tsumiba-selection':
      return run(() => sendSelection(info, tab));
    case 'tsumiba-link':
      return run(() => sendLink(info, tab));
    case 'tsumiba-image':
      return run(() => sendImage(info, tab));
    case 'tsumiba-page':
      return run(() => sendPage(tab));
    default:
      return undefined;
  }
}

async function activeTab() {
  const [tab] = await ext().tabs.query({ active: true, currentWindow: true });
  return tab;
}

export function onCommand(command, tab) {
  if (command !== 'send-page') return undefined;
  return run(async () => sendPage(tab ?? (await activeTab())));
}

// ---- wiring ----------------------------------------------------------------

// Listeners must be registered synchronously, at the top level, or the browser
// will not wake this script for the event.
const browserApi = ext();
browserApi.runtime.onInstalled.addListener(() => ensureMenus());
browserApi.runtime.onStartup?.addListener(() => ensureMenus());
(browserApi.contextMenus ?? browserApi.menus).onClicked.addListener(onMenuClicked);
browserApi.commands.onCommand.addListener(onCommand);

// Defensive: if the script was started for some other reason, make sure the
// menus exist.
ensureMenus();
clearBadge();
