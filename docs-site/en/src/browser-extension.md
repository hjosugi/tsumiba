# Browser extension

The extension adds a small button and a few right-click items to your browser that send
what you are looking at to the shelf: the **page**, a **selection**, a **link** or an
**image**. It works with **Chrome, Edge, Brave** (anything Chromium-based) and **Firefox**,
as a Manifest V3 extension.

It talks only to the tsumiba app on the same computer, over `127.0.0.1`. Nothing goes to
any server ([Privacy and security](privacy-and-security.md)).

> [!NOTE]
> The extension is not in the Chrome Web Store, Edge Add-ons or addons.mozilla.org yet.
> For now you load it from the zip on the release page. Because it is not signed by
> Mozilla, Firefox only keeps it until you restart the browser (below).

> [!WARNING]
> **How much has this been tested?** The extension was exercised in headless Chromium
> (service worker, menus, shortcut, popup and options pages) and the Firefox build was checked
> with `web-ext lint`. It has **never been run in Firefox, Edge or Brave**, nor on a
> real, non-headless Chrome. Expect rough edges and
> [please report them](https://github.com/hjosugi/tsumiba/issues).

## Install

Download the zip for your browser from the
[releases page](https://github.com/hjosugi/tsumiba/releases/latest):
`tsumiba-extension-chrome-<version>.zip` or `tsumiba-extension-firefox-<version>.zip`.

**Chrome, Edge, Brave**

1. Unzip it somewhere you will keep it (the browser reads the files from there).
2. Open `chrome://extensions` (`edge://extensions` in Edge, `brave://extensions` in Brave).
3. Turn on **Developer mode**.
4. Choose **Load unpacked** and select the unzipped folder.

**Firefox**

1. Open `about:debugging#/runtime/this-firefox`.
2. Choose **Load Temporary Add-on…** and select the zip (or the `manifest.json` inside the
   unzipped folder).
3. Temporary add-ons are removed when Firefox quits. Load it again after a restart until
   a signed version is published. Firefox release builds only keep add-ons that Mozilla has
   signed; for a permanent install today you need Firefox Developer Edition, Nightly or ESR
   with `xpinstall.signatures.required` set to `false`. Firefox 128 or newer is required.

Pin the extension's button to the toolbar so you can reach the popup.

**Building it yourself:** from a checkout, `node extension/build.mjs` (it needs the `zip`
program) writes `extension/dist/tsumiba-extension-chrome-<version>.zip`,
`extension/dist/tsumiba-extension-firefox-<version>.zip`, and the unzipped folders next to them.

## Pair it once

The extension needs the **token** that proves it is allowed to talk to your tsumiba. Without
it, nothing is sent.

1. Start tsumiba.
2. Get the token: run `tsumiba token` in a terminal, or open tsumiba →
   **Settings → Browser extension → Copy**.
3. Open the extension's **options** (right-click its button → *Options*, or open the popup →
   *Settings*), paste the token, check the **port** (default `47821`; it must match
   *Settings → Browser extension* in tsumiba), and choose **Save**.
4. Choose **Test connection**. It says *Connected to tsumiba 0.1.0* when it works.

The token is stored in the browser's extension storage and never leaves your computer. If
you regenerate it (`tsumiba token --regenerate`, or **Regenerate** in Settings), every
browser has to be paired again.

## Using it

### Right-click

| Right-click on… | Choose | What is saved |
| --- | --- | --- |
| selected text | **Send selection to tsumiba** | a new note: the text as a Markdown quote, then a link to the page |
| a link | **Send link to tsumiba** | a new note: the link text as a heading, the address, and "From:" the page |
| an image | **Send image to tsumiba** | the image itself, as a file on the shelf (its source is the page) |
| the page | **Send page to tsumiba** | a new note: the title as a heading, then the address |

The result shows on the toolbar button: a green ✓ for success, a red ! for a problem, with
the reason in its tooltip.

A note from a selection looks like this:

```markdown
> The shelf slides in from the top edge of the screen
> and goes away when you do.

[tsumiba documentation](https://hjosugi.github.io/tsumiba/en/)
```

### The popup

Click the toolbar button for:

- **Connection status**: connected (with version and counts), not paired, not running, wrong token.
- **Quick note**: type, then **Save note** (or the shortcut shown below the box).
- **Save this page** and **Save selection**.
- **Open shelf**, which asks the running tsumiba to show itself.

### Keyboard

<kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> sends the current page. You can change it in your
browser's extension shortcut settings (`chrome://extensions/shortcuts`; in Firefox,
the gear menu on `about:addons` → *Manage Extension Shortcuts*).

## Images from other sites

To save an image itself, the browser sometimes has to download it from the site it
came from, and an extension cannot do that without access to that site. The first time that
fails, the extension asks **once** for access to all sites, and uses it only when you choose
*Send image to tsumiba*. If you say no, it remembers, and saves the image's **address**
as a note instead of the picture. You can allow access later, or take it away, under
*Images from other sites* in the extension's options.

Before sending, the extension asks tsumiba for its upload limit (`storage.max_upload_mb`). An
image bigger than that is refused locally, with a clear message, instead of being sent only to
be turned away.

Images embedded in the page itself (`data:` URLs) are read directly. A `blob:` image belongs
to the page that made it and cannot be read from outside, so only its address is saved,
which is not much use beyond that page.

## Permissions

| Permission | Why |
| --- | --- |
| `contextMenus` | The four right-click items |
| `storage` | Remember the port and token in this browser profile |
| `activeTab`, `scripting` | Read the current tab's title and address (and the selection, from the popup) when you ask |
| `http://127.0.0.1/*`, `http://localhost/*` | Talk to tsumiba on this computer |
| *optional:* all sites | Fetch an image itself (see above); asked for only when needed |

The extension makes no other network requests: no analytics, no update pings of its own.

## Troubleshooting

| The popup or badge says | Meaning and fix |
| --- | --- |
| *Not paired yet* | No token saved. Follow *Pair it once*. |
| *tsumiba isn't running* | Start tsumiba. If it is running, check the port: if the default port was busy, tsumiba picked another and told you (**Settings → Browser extension** shows it); put that port in the options. |
| *The token doesn't match* | The token changed (regenerated, or a different data folder). Copy it again. |
| *That isn't tsumiba* | Another program answers on that port. Change the port in both places. |
| *tsumiba refused the request* | The server answered "forbidden". Check the port. |
| *Too large for tsumiba's upload limit* | Raise `storage.max_upload_mb` ([Configuration](configuration.md#storage)). |
| The shortcut does nothing | Another extension or the system may own <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd>. Change it in your browser's extension shortcut settings. |
| Firefox: sending fails with no clear reason | Firefox treats host permissions as optional. Open `about:addons` → tsumiba → *Permissions* and allow access to `127.0.0.1`. |
| An image arrives as a link, not a file | The browser could not fetch it and access to all sites was refused (or the site refuses scripts). Allow it under *Images from other sites* in the options. |

If you turned **Accept the browser extension and command line** off in tsumiba's
settings, the API is not running at all, and nothing here can connect.
