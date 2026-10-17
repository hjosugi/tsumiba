// Every user-visible string, in English and Japanese.
//
// A plain object per language rather than the browser's _locales machinery:
// the language follows `navigator.language`, works the same in every browser,
// and the unit tests can check that the two languages never drift apart.

export const en = {
  // connection states (popup)
  statusChecking: 'Checking…',
  statusOk: 'Connected',
  statusOkDetail: 'v{version} · Notes: {notes} · Files: {files}',
  statusNotPaired: 'Not paired yet',
  statusNotPairedDetail: 'Open Settings and paste your token.',
  statusNotRunning: "tsumiba isn't running",
  statusNotRunningDetail: 'Start tsumiba, then try again.',
  statusUnauthorized: "The token doesn't match",
  statusUnauthorizedDetail: 'Copy the token from tsumiba again.',
  statusNotTsumiba: "That isn't tsumiba",
  statusNotTsumibaDetail: 'Another program uses port {port}.',
  statusForbidden: 'tsumiba refused the request',
  statusForbiddenDetail: 'Check the port in Settings.',
  statusError: 'Something went wrong',

  // popup
  memoLabel: 'Quick note',
  memoPlaceholder: 'Write a note…',
  memoSave: 'Save note',
  memoHint: '{shortcut} to save',
  savePage: 'Save this page',
  saveSelection: 'Save selection',
  noSelection: 'Select some text on the page to save it',
  openShelf: 'Open shelf',
  openSettings: 'Settings',
  savedToTsumiba: 'Saved to tsumiba',
  cantReadPage: "This page can't be read (browser pages are off limits)",

  // feedback and errors
  sentOk: 'Sent to tsumiba',
  sentImageFile: 'Image saved to tsumiba',
  sentImageLink: "Saved the image's address (the image itself couldn't be fetched)",
  errNotPaired: 'Not paired yet: open the extension settings and add your token',
  errNotRunning: "tsumiba isn't running",
  errUnauthorized: 'Wrong token: pair again in the extension settings',
  errTooLarge: "Too large for tsumiba's upload limit",
  errTooLargeLimit: 'Too large: tsumiba accepts files up to {limit} MiB',
  errForbidden: 'tsumiba refused the request',
  errNotTsumiba: 'Another program is using the port',
  errBadRequest: 'tsumiba could not use that: {detail}',
  errOther: "Couldn't reach tsumiba: {detail}",
  errEmpty: 'Nothing to save',

  // context menus
  menuSelection: 'Send selection to tsumiba',
  menuLink: 'Send link to tsumiba',
  menuImage: 'Send image to tsumiba',
  menuPage: 'Send page to tsumiba',

  // options
  optionsTitle: 'tsumiba extension',
  optionsIntro: 'Sends pages, selections, links and images to the tsumiba app on this computer. Nothing leaves your machine.',
  portLabel: 'Port',
  portHelp: 'Default 47821. It must match Settings → Browser extension in tsumiba.',
  portInvalid: 'Enter a port from 1024 to 65535',
  tokenLabel: 'Token',
  tokenHelp: 'Run `tsumiba token` in a terminal, or open tsumiba → Settings → Browser extension → Copy, then paste it here.',
  tokenInvalid: 'The token is 64 characters: 0–9 and a–f',
  tokenShow: 'Show',
  tokenHide: 'Hide',
  save: 'Save',
  settingsSaved: 'Saved',
  testConnection: 'Test connection',
  testing: 'Testing…',
  pairingTitle: 'Pairing',
  pairingStep1: 'Start tsumiba.',
  pairingStep2: 'Run `tsumiba token`, or open tsumiba → Settings → Browser extension → Copy.',
  pairingStep3: 'Paste the token above, then Save and Test connection.',
  imagesTitle: 'Images from other sites',
  imagesHelp: 'To save an image itself (not just its address) the browser sometimes has to fetch it from the site it came from. That needs access to all sites; it is only used when you choose “Send image to tsumiba”.',
  imagesGrant: 'Allow',
  imagesRevoke: 'Remove access',
  imagesGranted: 'Allowed',
  imagesNotGranted: 'Not allowed: images from some sites are saved as links',

  // connection test results (options)
  testOk: 'Connected to tsumiba {version}. Notes: {notes}, files: {files}.',
  testNotPaired: 'tsumiba {version} answers on port {port}, but no token is set yet.',
  testNotRunning: "tsumiba isn't running on port {port}. Start it, or check the port.",
  testUnauthorized: 'tsumiba {version} is running, but it does not accept this token. Copy it again.',
  testNotTsumiba: 'Something other than tsumiba answers on port {port}.',
  testForbidden: 'tsumiba refused the request. Check the port.',
  testError: 'The test failed: {detail}',
};

export const ja = {
  statusChecking: '確認中…',
  statusOk: '接続しています',
  statusOkDetail: 'v{version} · メモ {notes} 件 · ファイル {files} 件',
  statusNotPaired: 'ペアリングが未設定です',
  statusNotPairedDetail: '設定を開いてトークンを貼り付けてください。',
  statusNotRunning: 'tsumiba が起動していません',
  statusNotRunningDetail: 'tsumiba を起動してからもう一度お試しください。',
  statusUnauthorized: 'トークンが一致しません',
  statusUnauthorizedDetail: 'tsumiba からトークンをもう一度コピーしてください。',
  statusNotTsumiba: 'tsumiba ではありません',
  statusNotTsumibaDetail: 'ポート {port} を別のプログラムが使っています。',
  statusForbidden: 'tsumiba に拒否されました',
  statusForbiddenDetail: '設定のポート番号を確認してください。',
  statusError: '問題が発生しました',

  memoLabel: 'クイックメモ',
  memoPlaceholder: 'メモを書く…',
  memoSave: 'メモを保存',
  memoHint: '{shortcut} で保存',
  savePage: 'このページを保存',
  saveSelection: '選択範囲を保存',
  noSelection: 'ページ上の文字を選択すると保存できます',
  openShelf: '積み場を開く',
  openSettings: '設定',
  savedToTsumiba: 'tsumiba に保存しました',
  cantReadPage: 'このページは読み取れません（ブラウザの内部ページです）',

  sentOk: 'tsumiba に送りました',
  sentImageFile: '画像を tsumiba に保存しました',
  sentImageLink: '画像のアドレスを保存しました（画像そのものは取得できませんでした）',
  errNotPaired: 'ペアリングが未設定です。拡張機能の設定でトークンを入力してください',
  errNotRunning: 'tsumiba が起動していません',
  errUnauthorized: 'トークンが違います。拡張機能の設定でペアリングし直してください',
  errTooLarge: 'tsumiba のアップロード上限を超えています',
  errTooLargeLimit: 'ファイルが大きすぎます。tsumiba が受け付けるのは {limit} MiB までです',
  errForbidden: 'tsumiba に拒否されました',
  errNotTsumiba: 'そのポートは別のプログラムが使っています',
  errBadRequest: 'tsumiba が受け付けられませんでした: {detail}',
  errOther: 'tsumiba に接続できませんでした: {detail}',
  errEmpty: '保存する内容がありません',

  menuSelection: '選択範囲を tsumiba に送る',
  menuLink: 'リンクを tsumiba に送る',
  menuImage: '画像を tsumiba に送る',
  menuPage: 'ページを tsumiba に送る',

  optionsTitle: 'tsumiba 拡張機能',
  optionsIntro: 'ページ・選択範囲・リンク・画像を、このコンピューター上の tsumiba に送ります。データはこのマシンの外に出ません。',
  portLabel: 'ポート',
  portHelp: '既定は 47821 です。tsumiba の「設定 → ブラウザ拡張」と同じ値にしてください。',
  portInvalid: '1024〜65535 のポート番号を入力してください',
  tokenLabel: 'トークン',
  tokenHelp: 'ターミナルで `tsumiba token` を実行するか、tsumiba の「設定 → ブラウザ拡張 → コピー」で取得して貼り付けてください。',
  tokenInvalid: 'トークンは 0〜9 と a〜f の 64 文字です',
  tokenShow: '表示',
  tokenHide: '隠す',
  save: '保存',
  settingsSaved: '保存しました',
  testConnection: '接続をテスト',
  testing: 'テスト中…',
  pairingTitle: 'ペアリング',
  pairingStep1: 'tsumiba を起動します。',
  pairingStep2: '`tsumiba token` を実行するか、tsumiba の「設定 → ブラウザ拡張 → コピー」を使います。',
  pairingStep3: '上の欄にトークンを貼り付け、「保存」→「接続をテスト」を押します。',
  imagesTitle: '他サイトの画像',
  imagesHelp: '画像そのもの（アドレスだけでなく）を保存するには、ブラウザが画像の取得元サイトから直接読み込む必要がある場合があります。そのためすべてのサイトへのアクセス許可が必要で、「画像を tsumiba に送る」を選んだときだけ使われます。',
  imagesGrant: '許可する',
  imagesRevoke: '許可を取り消す',
  imagesGranted: '許可されています',
  imagesNotGranted: '許可されていません。一部のサイトの画像はリンクとして保存されます',

  testOk: 'tsumiba {version} に接続できました。メモ {notes} 件、ファイル {files} 件。',
  testNotPaired: 'ポート {port} で tsumiba {version} が応答していますが、トークンが未設定です。',
  testNotRunning: 'ポート {port} で tsumiba が起動していません。起動するか、ポートを確認してください。',
  testUnauthorized: 'tsumiba {version} は起動していますが、このトークンは受け付けられません。もう一度コピーしてください。',
  testNotTsumiba: 'ポート {port} で応答しているのは tsumiba ではありません。',
  testForbidden: 'tsumiba に拒否されました。ポートを確認してください。',
  testError: 'テストに失敗しました: {detail}',
};

export const languages = { en, ja };

/** `ja`, `ja-JP`, `JA_jp` -> 'ja'; anything else -> 'en'. */
export function pickLang(tag) {
  return /^ja\b/i.test(String(tag ?? '').replace('_', '-')) ? 'ja' : 'en';
}

/** Look up `key` in `lang`, filling `{name}` placeholders from `params`. */
export function t(lang, key, params = {}) {
  const table = languages[lang] ?? en;
  const template = table[key] ?? en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (whole, name) => (name in params ? String(params[name]) : whole));
}

/** The names of the `{placeholders}` in a template. */
export function placeholders(template) {
  return [...String(template).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

/** The message for an error from api.js. */
export function errorMessage(lang, error) {
  const detail = error?.message ?? '';
  switch (error?.code) {
    case 'not-running':
      return t(lang, 'errNotRunning');
    case 'unauthorized':
      return t(lang, 'errUnauthorized');
    case 'too-large':
      return error.limitBytes
        ? t(lang, 'errTooLargeLimit', { limit: Math.round(error.limitBytes / (1024 * 1024)) })
        : t(lang, 'errTooLarge');
    case 'forbidden':
      return t(lang, 'errForbidden');
    case 'not-tsumiba':
      return t(lang, 'errNotTsumiba');
    case 'bad-request':
      return t(lang, 'errBadRequest', { detail });
    case 'not-paired':
      return t(lang, 'errNotPaired');
    default:
      return t(lang, 'errOther', { detail });
  }
}

/** Fill `data-i18n*` attributes in a document. */
export function applyI18n(root, lang) {
  for (const el of root.querySelectorAll('[data-i18n]')) el.textContent = t(lang, el.dataset.i18n);
  for (const el of root.querySelectorAll('[data-i18n-placeholder]')) {
    el.setAttribute('placeholder', t(lang, el.dataset.i18nPlaceholder));
  }
  for (const el of root.querySelectorAll('[data-i18n-title]')) {
    el.setAttribute('title', t(lang, el.dataset.i18nTitle));
  }
  for (const el of root.querySelectorAll('[data-i18n-aria-label]')) {
    el.setAttribute('aria-label', t(lang, el.dataset.i18nAriaLabel));
  }
  root.documentElement?.setAttribute('lang', lang);
}
