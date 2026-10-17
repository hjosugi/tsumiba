# Security policy

## Reporting a vulnerability

Please report security problems privately through GitHub's
[private vulnerability reporting](https://github.com/hjosugi/tsumiba/security/advisories/new)
rather than in a public issue. You will get an answer within a few days.

Only the latest release is supported.

## What tsumiba exposes

tsumiba opens one network listener: an HTTP API on `127.0.0.1` (default port 47821),
used by the command line and the browser extension. It never listens on another
interface and makes no outgoing connections.

Because any web page can make *your browser* send requests to `127.0.0.1`, loopback
alone is not protection. Every request has to pass three checks:

1. The `Host` header must be `127.0.0.1:<port>`, `localhost:<port>` or `[::1]:<port>`
   (refuses DNS rebinding).
2. If an `Origin` header is present it must be a browser-extension origin
   (`chrome-extension://`, `moz-extension://`, `safari-web-extension://`). Ordinary
   web pages are refused before anything is read.
3. A bearer token that only the app's own files and the extension's storage contain.
   The token file is created with owner-only permissions. `GET /v1/ping` is the one
   endpoint that needs no token and reveals only the program name and version.

Uploads have a size limit enforced while streaming (so it holds without a
`Content-Length`), file names are reduced to a single safe path component, and ids
arriving over HTTP are validated before they touch the filesystem.

If you find a way around any of this, that is exactly the kind of report we want.

## What tsumiba does not protect

- Notes and files are stored unencrypted in your user profile, like any other document.
- Anyone who can read your user profile can read the token, and so can use the API.
- Releases are not yet code-signed (Windows) or notarised (macOS).
