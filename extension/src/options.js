// The options page: where to find tsumiba and how to pair with it.

import { checkConnection } from './lib/api.js';
import { ext } from './lib/ext.js';
import { applyI18n, pickLang, t } from './lib/i18n.js';
import { isValidToken, loadSettings, normalizeToken, parsePort, saveSettings } from './lib/settings.js';

const lang = pickLang(navigator.language);
applyI18n(document, lang);

const ALL_SITES = { origins: ['<all_urls>'] };
const $ = (id) => document.getElementById(id);

// ---- reading and validating the form ---------------------------------------

function showError(field, text) {
  const error = $(`${field}-error`);
  error.textContent = text ?? '';
  error.hidden = !text;
  $(field).setAttribute('aria-invalid', text ? 'true' : 'false');
}

/** The form's values, or `null` (with the problems shown) when they are not usable. */
function readForm() {
  const port = parsePort($('port').value);
  const token = normalizeToken($('token').value);
  showError('port', port === null ? t(lang, 'portInvalid') : null);
  showError('token', token !== '' && !isValidToken(token) ? t(lang, 'tokenInvalid') : null);
  if (port === null || (token !== '' && !isValidToken(token))) return null;
  return { port, token };
}

// ---- save and test -----------------------------------------------------------

function showResult(state, text) {
  const result = $('test-result');
  result.dataset.state = state;
  result.textContent = text;
  result.hidden = false;
}

const RESULT_VIEW = {
  ok: ['ok', 'testOk'],
  'not-paired': ['warn', 'testNotPaired'],
  'not-running': ['bad', 'testNotRunning'],
  unauthorized: ['bad', 'testUnauthorized'],
  'not-tsumiba': ['bad', 'testNotTsumiba'],
  forbidden: ['bad', 'testForbidden'],
};

async function testConnection(values) {
  const button = $('test');
  button.disabled = true;
  showResult('warn', t(lang, 'testing'));
  try {
    const result = await checkConnection(values);
    const [tone, key] = RESULT_VIEW[result.state] ?? ['bad', 'testError'];
    showResult(
      tone,
      t(lang, key, {
        version: result.version ?? '',
        notes: result.notes ?? 0,
        files: result.files ?? 0,
        port: values.port,
        detail: result.error?.message ?? '',
      }),
    );
  } finally {
    button.disabled = false;
  }
}

$('form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const values = readForm();
  if (!values) return;
  try {
    await saveSettings(values);
  } catch (error) {
    showError('token', error.message);
    return;
  }
  const saved = $('saved');
  saved.textContent = t(lang, 'settingsSaved');
  saved.hidden = false;
  setTimeout(() => (saved.hidden = true), 2500);
  // The token field now holds the tidied value.
  $('token').value = values.token;
  await testConnection(values);
});

$('test').addEventListener('click', async () => {
  const values = readForm();
  if (values) await testConnection(values);
});

$('toggle-token').addEventListener('click', () => {
  const field = $('token');
  const show = field.type === 'password';
  field.type = show ? 'text' : 'password';
  $('toggle-token').setAttribute('aria-pressed', String(show));
  $('toggle-token').textContent = t(lang, show ? 'tokenHide' : 'tokenShow');
});

for (const id of ['port', 'token']) $(id).addEventListener('input', () => showError(id, null));

// ---- permission to fetch images from any site -------------------------------

async function renderImagePermission() {
  let granted = false;
  try {
    granted = await ext().permissions.contains(ALL_SITES);
  } catch {
    granted = false;
  }
  const pill = $('images-state');
  pill.dataset.state = granted ? 'on' : 'off';
  pill.textContent = t(lang, granted ? 'imagesGranted' : 'imagesNotGranted');
  $('images-grant').hidden = granted;
  $('images-revoke').hidden = !granted;
}

// `permissions.request` only works inside a click, which these are.
$('images-grant').addEventListener('click', async () => {
  try {
    const granted = await ext().permissions.request(ALL_SITES);
    if (granted) await saveSettings({ imagePermissionDenied: false });
  } finally {
    await renderImagePermission();
  }
});

$('images-revoke').addEventListener('click', async () => {
  try {
    await ext().permissions.remove(ALL_SITES);
  } finally {
    await renderImagePermission();
  }
});

// ---- start ------------------------------------------------------------------

async function start() {
  const settings = await loadSettings();
  $('port').value = String(settings.port);
  $('token').value = settings.token;
  await renderImagePermission();
  if (settings.token) await testConnection(settings);
}

start();
