# HTTP API

While the app runs (or `tsumiba serve`), it answers a small HTTP API on your own computer.
The [command line](cli.md) and the [browser extension](browser-extension.md) are its two
clients, and you can be a third.

- **Address:** `http://127.0.0.1:<port>/v1`, default port **47821** (`api.port`).
  `127.0.0.1` only; it is never reachable from the network.
- **Format:** JSON in, JSON out, UTF-8. Errors are `{"error": "message"}` with an HTTP
  status that says what kind.
- **Authentication:** `Authorization: Bearer <token>` on everything except `GET /ping`.
  The token is in `config/token` and printed by `tsumiba token`.
- **Version:** all paths start with `/v1`. Fields only ever get added; ignore ones you do
  not know.

```sh
TOKEN=$(tsumiba token)
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:47821/v1/status
# {"notes":3,"files":7,"visible":false,"max_upload_mb":512}
```

If the default port was taken when tsumiba started, it picked another and recorded it in
`config/server.json` (`{"port": …, "pid": …, "version": …}`); the command line reads
that file, other clients need to be told the port.

## Security model

Any web page you visit can make your browser send requests to `127.0.0.1`, so listening
on loopback alone is not protection. Every request passes three checks, in
this order, before anything is read or done:

1. **`Host`** must be `127.0.0.1:<port>`, `localhost:<port>` or `[::1]:<port>`. Anything
   else is `403 unexpected Host header`. This defeats *DNS rebinding*, where an attacker's
   domain is made to resolve to `127.0.0.1`.
2. **`Origin`**, when the request has one, must be a **browser-extension** origin:
   `chrome-extension://…`, `moz-extension://…` or `safari-web-extension://…`. An ordinary
   web page's origin (`https://example.com`, `http://localhost`, `null`) is
   `403 this origin is not allowed`, even with a valid token. Requests without an `Origin`
   (curl, scripts) pass this check.
3. **The token.** `Authorization: Bearer <64 hex digits>`, compared in constant time.
   Missing or wrong is `401 missing or wrong token`. The token never reaches a web page;
   the extension keeps it in its own storage.

Beyond that:

- The only unauthenticated endpoint is `GET /v1/ping`, which reveals nothing but that this
  is tsumiba and its version, so a client can tell it from some other program that holds
  the port. It still has to pass the `Host` and `Origin` checks.
- **CORS:** for an allowed extension origin the server answers with
  `Access-Control-Allow-Origin: <that origin>` and `Vary: Origin`. A preflight (`OPTIONS`)
  from an allowed origin gets `204` with `Access-Control-Allow-Methods: GET, POST, PATCH,
  DELETE, OPTIONS`, `Access-Control-Allow-Headers: authorization, content-type,
  x-tsumiba-name, x-tsumiba-source`, and `Access-Control-Max-Age: 600`; it needs no token,
  because browsers never send one on a preflight. A preflight from any other origin is
  `403` and gets no CORS headers at all.
- Responses carry `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`.
- **Sizes:** a JSON body may be up to 4 MiB plus a little (`413` beyond that); an upload
  is limited by `storage.max_upload_mb` (default 512 MiB). The limit is checked up front
  from `Content-Length` and again while the body streams in, so a client that lies about
  the length, or sends no length (chunked), is cut off at the limit and nothing is left on
  disk.
- File names are sanitised: only the last path component is kept, characters Windows
  forbids become `_`, reserved names such as `CON` are prefixed, the length is capped. A
  name like `../../etc/cron.d/x` becomes `x` inside the shelf.
- Ids in URLs must be exactly 20 lowercase hex digits; anything else cannot reach the file
  system.

## Data shapes

**Note**

```json
{
  "id": "01a148d44c055be0439c",
  "title": "Release checklist",
  "body": "# Release checklist\n- [ ] tag it\n",
  "pinned": true,
  "created_ms": 1760719443000,
  "updated_ms": 1760719443000
}
```

`title` is the first non-empty line without leading `#`s. Times are milliseconds since the
Unix epoch. `created_ms` is part of the id; `updated_ms` is the file's modification time.

**File**

```json
{
  "id": "01a148d44c5b4e5d2364",
  "name": "screenshot.png",
  "size": 2032,
  "added_ms": 1760719443000,
  "pinned": false,
  "source": "https://example.com/page",
  "link": false,
  "path": "/home/me/.local/share/tsumiba/files/01a148d44c5b4e5d2364/screenshot.png"
}
```

`source` is where it came from when known (a page address for browser uploads, the original
path for local imports) and may be `null`. `link` is true when only the location is
remembered; `path` is then the original. `size` of a folder link is `0`.

## Endpoints

| Method and path | Auth | Does |
| --- | --- | --- |
| `GET /v1/ping` | no | `{"app":"tsumiba","version":"0.1.0"}` |
| `GET /v1/status` | yes | `{"notes": n, "files": n, "visible": bool, "max_upload_mb": n}` |
| `POST /v1/window` | yes | Show, hide or toggle the shelf. `204` |
| `POST /v1/quit` | yes | Quit tsumiba. `204` |
| `GET /v1/notes` | yes | List notes (pinned first, then newest first) |
| `POST /v1/notes` | yes | Create a note. `201` |
| `GET /v1/notes/{id}` | yes | One note, or `404` |
| `PATCH /v1/notes/{id}` | yes | Change a note's text and/or pin. `200` |
| `DELETE /v1/notes/{id}` | yes | Delete a note (to the trash). `204` |
| `GET /v1/files` | yes | List files (pinned first, then newest first) |
| `POST /v1/files` | yes | Upload a file (raw body). `201` |
| `POST /v1/files/import` | yes | Import files from paths on this machine |
| `DELETE /v1/files/{id}` | yes | Remove a file (a copy to the trash; a link is forgotten). `204` |
| `OPTIONS *` | no | CORS preflight for extension origins. `204` |

### `GET /v1/status`

`notes` and `files` are counts; `visible` is whether the shelf is on screen right now (always
`false` under `tsumiba serve`); `max_upload_mb` is the largest upload the server accepts, in
MiB (`storage.max_upload_mb`, 512 by default). A client that uploads files should read it
first and refuse an oversized file itself: when the server rejects an upload early it answers
`413` and closes the connection while the body is still being sent, and on some systems the
client then sees a network error instead of the `413`. The browser extension does exactly this.

### `POST /v1/window`

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"action":"toggle"}' http://127.0.0.1:47821/v1/window
```

`action` is `"show"`, `"hide"` or `"toggle"`; anything else is `400`.

### Notes

```sh
# list, optionally filtered ("q" matches the text, case-insensitively) and limited
curl -s -H "Authorization: Bearer $TOKEN" 'http://127.0.0.1:47821/v1/notes?q=release&limit=10'

# create
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"text":"# Hello\nfrom curl","pinned":true}' http://127.0.0.1:47821/v1/notes

# change: both fields are optional; absent ones are left alone
curl -s -X PATCH -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"text":"new text","pinned":false}' http://127.0.0.1:47821/v1/notes/01a148d44c055be0439c
```

`POST` takes `{"text": string, "pinned": bool?}`. `text` is required and may be up to 4 MiB.
`PATCH` takes `{"text": string?, "pinned": bool?}`.

### Files

**Upload** takes the file's bytes as the request body. The name and origin go in headers,
percent-encoded as UTF-8 (everything outside `A-Z a-z 0-9 . _ ~ -` becomes `%XX`):

| Header | Meaning |
| --- | --- |
| `X-Tsumiba-Name` | The file name. Defaults to `file` |
| `X-Tsumiba-Source` | Where it came from, usually a page address. Optional |
| `Content-Type` | Used only to add an extension when the name has none (`image/png` → `.png`, also jpeg, gif, webp, svg, avif, pdf, txt, html, json, zip) |

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H 'X-Tsumiba-Name: report%202026.pdf' \
  -H 'X-Tsumiba-Source: https%3A%2F%2Fexample.com%2Freports' \
  -H 'Content-Type: application/pdf' \
  --data-binary @report.pdf http://127.0.0.1:47821/v1/files
```

Too big is `413`.

**Import** asks tsumiba to read files that are already on the machine it runs on, which
saves sending the bytes through HTTP (this is what `tsumiba add` uses):

```sh
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"paths":["/home/me/a.txt","/home/me/photos"],"link":false}' \
  http://127.0.0.1:47821/v1/files/import
# {"added":[{...}],"failed":[{"path":"...","error":"..."}]}
```

Paths must be absolute (a relative one is reported in `failed`). `link: true` stores links
instead of copies; leaving it out follows `storage.mode`. Folders are always links. The
status is `200` even when some paths failed: look at `failed`.

### Errors

| Status | When |
| --- | --- |
| `400` | Malformed JSON, an unknown `action`, an invalid id |
| `401` | No token, or the wrong one |
| `403` | Wrong `Host`, or an `Origin` that is not a browser extension |
| `404` | No such note or file, or no such route |
| `413` | A body over the limit |
| `500` | Something failed on disk; the message says what |

## Compatibility

The API is versioned by its path. The intent is that within `/v1` only additions happen:
new endpoints, new response fields. Clients should ignore fields they do not know.
