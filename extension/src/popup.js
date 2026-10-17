// The toolbar popup: connection state, a quick-memo box, and buttons to save
// the current page or selection.

import { checkConnection, createClient } from './lib/api.js';
import { ext } from './lib/ext.js';
import { memoNote, pageNote, selectionNote } from './lib/format.js';
import { applyI18n, errorMessage, pickLang, t } from './lib/i18n.js';
import { loadSettings } from './lib/settings.js';

const lang = pickLang(navigator.language);
applyI18n(document, lang);

const $ = (id) => document.getElementById(id);
const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
$('memo-hint').textContent = t(lang, 'memoHint', { shortcut: isMac ? '⌘↵' : 'Ctrl+Enter' });

let settings;
let tab = null;
let selection = '';

// ---- connection state -------------------------------------------------------

const STATE_VIEW = {
  ok: { tone: 'ok', title: 'statusOk', detail: 'statusOkDetail' },
  'not-paired': { tone: 'warn', title: 'statusNotPaired', detail: 'statusNotPairedDetail' },
  'not-running': { tone: 'bad', title: 'statusNotRunning', detail: 'statusNotRunningDetail' },
  unauthorized: { tone: 'bad', title: 'statusUnauthorized', detail: 'statusUnauthorizedDetail' },
  'not-tsumiba': { tone: 'bad', title: 'statusNotTsumiba', detail: 'statusNotTsumibaDetail' },
  forbidden: { tone: 'bad', title: 'statusForbidden', detail: 'statusForbiddenDetail' },
  error: { tone: 'bad', title: 'statusError', detail: null },
};

function renderState(result) {
  const view = STATE_VIEW[result.state] ?? STATE_VIEW.error;
  $('status').dataset.state = view.tone;
  $('status-title').textContent = t(lang, view.title);
  $('status-detail').textContent = view.detail
    ? t(lang, view.detail, {
        version: result.version ?? '',
        notes: result.notes ?? 0,
        files: result.files ?? 0,
        port: settings.port,
      })
    : (result.error?.message ?? '');
  // Without a token every save would fail; with one, let the user try (the
  // error says exactly what is wrong).
  const blocked = result.state === 'not-paired';
  for (const id of ['save-memo', 'save-page']) $(id).disabled = blocked;
  $('save-selection').disabled = blocked || !selection.trim();
}

// ---- the active page --------------------------------------------------------

async function readActivePage() {
  const api = ext();
  try {
    [tab] = await api.tabs.query({ active: true, currentWindow: true });
  } catch {
    tab = null;
  }
  if (!tab?.id) return;
  try {
    const [injected] = await api.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => String(globalThis.getSelection?.() ?? ''),
    });
    selection = injected?.result ?? '';
    const hint = $('selection-hint');
    if (!selection.trim()) {
      hint.textContent = t(lang, 'noSelection');
      hint.hidden = false;
    }
  } catch {
    // chrome://, the add-ons store, a PDF viewer …: not scriptable.
    selection = '';
    const hint = $('selection-hint');
    hint.textContent = t(lang, 'cantReadPage');
    hint.hidden = false;
  }
}

// ---- saving -----------------------------------------------------------------

function showMessage(kind, text) {
  const message = $('message');
  message.dataset.kind = kind;
  message.textContent = text;
  message.hidden = false;
}

async function submit(button, work) {
  button.disabled = true;
  $('message').hidden = true;
  try {
    await work(createClient(settings));
    showMessage('ok', t(lang, 'savedToTsumiba'));
    setTimeout(() => window.close(), 600);
  } catch (error) {
    showMessage('error', errorMessage(lang, error));
    button.disabled = false;
  }
}

async function saveMemo() {
  const text = memoNote($('memo').value);
  if (!text) {
    showMessage('error', t(lang, 'errEmpty'));
    $('memo').focus();
    return;
  }
  await submit($('save-memo'), (client) => client.createNote(text));
}

$('save-memo').addEventListener('click', saveMemo);
$('memo').addEventListener('keydown', (event) => {
  if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    saveMemo();
  }
});

$('save-page').addEventListener('click', () =>
  submit($('save-page'), (client) => client.createNote(pageNote({ title: tab?.title, url: tab?.url }))),
);

$('save-selection').addEventListener('click', () =>
  submit($('save-selection'), (client) =>
    client.createNote(selectionNote({ selection, title: tab?.title, url: tab?.url })),
  ),
);

$('open-options').addEventListener('click', () => {
  ext().runtime.openOptionsPage();
  window.close();
});

$('open-shelf').addEventListener('click', async () => {
  try {
    await createClient(settings).window('show');
    window.close();
  } catch (error) {
    showMessage('error', errorMessage(lang, error));
  }
});

// ---- start ------------------------------------------------------------------

async function start() {
  settings = await loadSettings();
  $('memo').focus();
  // Both are independent: ask the page and the app at the same time.
  const [, connection] = await Promise.all([readActivePage(), checkConnection(settings)]);
  renderState(connection);
}

start();
