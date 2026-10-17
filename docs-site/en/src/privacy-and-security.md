# Privacy and security

## Privacy

- **No account, no cloud, no telemetry.** tsumiba never sends your notes, your files or
  anything about you anywhere.
- **No outgoing connections.** The program contains an HTTP client, but it is used only to
  reach *your own* tsumiba on `127.0.0.1` (the command line talking to the app). It does not check for
  updates and does not contact any server. Updating is something you do on purpose.
- **The browser extension** talks only to `127.0.0.1`. It reads the page's title,
  address and your selection only when you ask it to send them.
- **This documentation site** makes no third-party requests either: no web fonts, no
  analytics, no script from elsewhere.
- Your data is plain files in a folder you choose ([Data and sync](data-and-sync.md)).
  It is **not encrypted**; it is as private as the rest of your home folder. Use full-disk
  encryption if that matters to you.

## What listens, and what protects it

When the app runs it listens on **`127.0.0.1:47821`** (configurable, or off with
`api.enabled = false`), for the command line and the browser extension. It is on the loopback
interface only, so other computers cannot reach it.

Loopback is still reachable by every program on your computer, and by every web page you
open, through your browser. So the API is guarded ([HTTP API](api.md#security-model) has the details):

| Layer | Stops |
| --- | --- |
| The **`Host`** header must name loopback and the right port | DNS rebinding: a malicious site whose domain is switched to `127.0.0.1` |
| The **`Origin`** header, if present, must be a browser extension | Any ordinary web page, even one that somehow knew the token |
| A **bearer token**, 256 bits from the system's random generator, compared in constant time | Everything else: other local users, other programs without file access, guessing |

The token lives in `token` in the configuration folder, readable only by you on Linux and
macOS. It is never sent to a web page; the extension keeps its copy in its own storage.
`tsumiba token --regenerate` replaces it.

## What the token is worth

**Treat the token like a password for your shelf.** Whoever has it can read, add, change and
delete your notes and files through the API, and, because the API can *import* files by
path, can ask tsumiba to copy any file *you* can read into the shelf and then read it back.
That is exactly as much as any program running as you could do directly, which is the real
boundary here:

- A program running **as you** can read your notes folder, and your token file, without
  tsumiba's help. tsumiba does not try to defend against that; neither can any desktop app.
- **Other users** on the same machine can connect to the port, but cannot read your token
  (permissions `0600` on Linux and macOS), so they get `401`.
- **Web pages** are stopped by the Host and Origin checks even before the token is
  looked at.

## Limits and honest caveats

- The data folder is not encrypted and tsumiba does not change its permissions; they are
  whatever your system gives new files. Make the folder private yourself if the machine is shared.
- On **Windows** the token file lives in your profile, which Windows already restricts to
  you; no extra permissions are set.
- Releases are **not code-signed or notarised yet**. Check downloads against
  `SHA256SUMS.txt`; the installer scripts do this for you.
- Local programs can see that something listens on the port (`GET /v1/ping` is open by
  design, so clients can tell tsumiba from another program). It reveals the name and
  version, nothing else.
- The extension's optional "all sites" permission, if you grant it, is used only to
  download an image you chose to send.

## Reporting a problem

Please report security problems privately, through GitHub's
[private vulnerability reporting](https://github.com/hjosugi/tsumiba/security/advisories/new),
so that they can be fixed before they are public (the repository's
[security policy](https://github.com/hjosugi/tsumiba/blob/main/SECURITY.md) has the details).
Ordinary bugs are welcome as [issues](https://github.com/hjosugi/tsumiba/issues); the output
of `tsumiba paths` and the last lines of the log file make a report much more useful.
