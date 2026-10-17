# tsumiba browser extension

Send what you are looking at to your [tsumiba](https://hjosugi.github.io/tsumiba/)
shelf: a page, a selection, a link, an image, or a quick note typed into the
toolbar popup. It works with Chrome, Edge, Brave and other Chromium browsers,
and with Firefox 128 or newer.

The extension talks only to the tsumiba app on **your own computer**
(`127.0.0.1`). It has no server, no account and no analytics.

## What it does

| You do | tsumiba gets |
| --- | --- |
| Right-click a page → **Send page to tsumiba** | a note: `# Title` and the address |
| Select text, right-click → **Send selection to tsumiba** | a note quoting the text, linking back to the page |
| Right-click a link → **Send link to tsumiba** | a note with the link and where you found it |
| Right-click an image → **Send image to tsumiba** | the image as a file on the shelf (or, if the browser cannot fetch it, a note with its address) |
| Press **Alt+Shift+S** | the current page, as a note |
| Click the toolbar button | a popup with a quick-note box, *Save this page* and *Save selection* |

A green ✓ on the toolbar button means it arrived; a red ! means it did not, and
hovering the button says why. Notes are plain Markdown, for example:

```markdown
> The quoted selection,
> line by line.

[Page title](https://example.com/article)
```

The interface follows your browser's language (English or Japanese) and its
light/dark setting.

## Install

There are no store listings yet, so you install it by hand from a release
(`tsumiba-extension-chrome-<version>.zip`, `tsumiba-extension-firefox-<version>.zip`)
or from a build of this repository (`node extension/build.mjs`, output in
`extension/dist/`).

### Chrome, Edge, Brave

1. Unzip `tsumiba-extension-chrome-<version>.zip` somewhere permanent. The
   browser reads the files from that folder every time it starts.
2. Open `chrome://extensions` (Edge: `edge://extensions`, Brave: `brave://extensions`).
3. Turn on **Developer mode**.
4. Choose **Load unpacked** and pick the unzipped folder.

### Firefox

1. Open `about:debugging#/runtime/this-firefox`.
2. Choose **Load Temporary Add-on…** and pick `manifest.json` from the unzipped
   Firefox build (or the `.zip` itself).

Be aware of what this means: a temporary add-on is **removed when Firefox
quits**, and Firefox release builds only keep add-ons that Mozilla has signed.
tsumiba's add-on has not been submitted to addons.mozilla.org yet, so for a
permanent install today you need Firefox Developer Edition, Nightly or ESR with
`xpinstall.signatures.required` set to `false`. Signed distribution is future work.

## Pair it with the app

The extension and the app share a secret token, so that no website can talk to
your shelf.

1. Start tsumiba.
2. Get the token, either by running `tsumiba token` in a terminal, or in the app:
   **Settings → Browser extension → Copy**.
3. Open the extension's settings (toolbar popup → *Settings*), paste the token,
   check the port (default `47821`), press **Save**.
4. **Test connection** should say *Connected to tsumiba …*.

If you press **Regenerate** in the app, every browser has to be paired again.

## Permissions, and why

| Permission | What it is used for |
| --- | --- |
| `contextMenus` | the right-click items |
| `storage` | remembering the port and the token, on this device only |
| `activeTab` | reading the title and address of the page you are on when you click the toolbar button or press the shortcut; no standing access to your tabs |
| `scripting` | reading your current text selection on the active page, when the popup opens |
| `http://127.0.0.1/*`, `http://localhost/*` | talking to the tsumiba app (a match pattern cannot name a port, so the port is chosen in settings) |
| optional: all sites (`<all_urls>`) | **only** to download an image from the site it came from when the browser would otherwise block it. It is requested the first time that is needed, can be refused (the image's address is saved instead, and you are not asked again), and can be granted or removed any time in the settings |

There are no content scripts: nothing is injected into pages unless you ask for it.

## Privacy

The extension sends data to exactly one place: the tsumiba app listening on
`127.0.0.1` on this computer. The one other network request it can make is the
download of an image you explicitly chose to send. Nothing is sent to the
extension's authors or any third party, and there is no telemetry.

## Troubleshooting

- **"tsumiba isn't running"** — start the app. If it is running, check the port:
  Settings → Browser extension in the app shows the port in use. The default
  `47821` can be taken by another program; the app then picks a free one and says
  so. Put that number in the extension's settings, or fix a port in tsumiba's
  `config.toml` (`[api] port = …`) and restart it.
- **"The token doesn't match"** — copy the token again (see *Pair it*). It is 64
  characters, `0–9` and `a–f`; stray spaces and line breaks are removed for you.
- **"Another program is using the port"** — something else answers on that port.
  Choose another port in both places.
- **Firefox asks about access to 127.0.0.1** — Firefox treats host permissions
  as optional. If sending fails, open `about:addons` → tsumiba → *Permissions*
  and allow access to `127.0.0.1`.
- **Brave** — Shields and the "localhost access" prompts apply to websites, not
  to extensions. If a content blocker rule blocks `127.0.0.1`, allow it for the
  extension.
- **An image arrives as a link, not a file** — the browser could not fetch it and
  access to all sites was refused (or the site returns an error to scripts).
  Grant access in the extension's settings → *Images from other sites*.
- **The Alt+Shift+S shortcut does nothing** — another extension or the system may
  own it. Change it at `chrome://extensions/shortcuts` (Firefox: `about:addons` →
  gear icon → *Manage Extension Shortcuts*).
- **Too large** — tsumiba refuses files over its upload limit (512 MiB by default,
  `[storage] max_upload_mb` in `config.toml`).

## For developers

The extension is plain JavaScript (ES modules), with no dependencies and no build
step beyond copying files and writing the right manifest for each browser.

```sh
node build.mjs            # dist/chrome, dist/firefox and both .zip files
npm test                  # unit + integration: deterministic, needs a tsumiba binary
npm run test:browser      # real Chromium (set CHROMIUM_BIN, or have `chromium` on PATH)
npm run test:all          # everything
```

(On Node 24 `node --test test/` with a bare directory does not work; the scripts pass globs.)

The tests, from fastest to most real:

1. `test/unit` — note formatting, image naming, header encoding (the same vectors as
   the Rust tests), error mapping, translations, manifest validity.
2. `test/integration` — the client and `background.js` against the real
   `tsumiba serve`, with a fake `chrome` namespace standing in for the browser.
   These need a debug build of tsumiba (`cargo build -p tsumiba`; or set
   `TSUMIBA_BIN`) and are skipped, with a message, without one.
3. `test/browser` — the built extension loaded into headless Chromium and driven over
   the DevTools Protocol: service worker, menus, shortcut, popup, options page,
   and a check that an ordinary web page cannot use the API. Needs `chromium` on
   `PATH` (or `CHROMIUM_BIN`), e.g.
   `nix shell nixpkgs#chromium -c npm run test:browser`. A Firefox build is only
   linted (`web-ext lint`), not run.

The wire contract is documented with the app: *HTTP API*.
