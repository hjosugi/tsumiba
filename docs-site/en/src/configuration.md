# Configuration

Settings live in one file, `config.toml`. You can change most of them in the app
(**Settings**, the gear button, or <kbd>Ctrl</kbd>/<kbd>⌘</kbd>+<kbd>,</kbd>) and all of
them by editing the file. `tsumiba paths` prints where the file is.

- **Missing file, missing keys:** defaults. You never need to write a complete file.
- **Unknown keys:** ignored, so a file from a newer version still loads.
- **Out-of-range numbers:** clamped to a sensible range.
- **When it is read:** at start-up. Hand edits take effect after you restart tsumiba
  (`tsumiba quit`, then `tsumiba`).
- **When it is written:** when you change a setting in the app. The app rewrites the whole
  file, so comments you added by hand are lost; the values are kept.
- **A broken file** (a typo that is not valid TOML, or a wrong type such as
  `mode = "teleport"`) is reported when tsumiba starts, and tsumiba runs on the defaults
  for that session instead of refusing to start. The broken file itself is left alone until you
  change a setting in the app, which writes a fresh one, so **fix the typo before touching
  Settings.**

## Reference

### `[general]`

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `language` | `"auto"`, `"en"`, `"ja"` | `"auto"` | Interface language. `auto` follows the system locale: Japanese for `ja…`, English otherwise |
| `theme` | `"system"`, `"light"`, `"dark"` | `"system"` | Colour theme. `system` follows the operating system |
| `launch_at_login` | boolean | `false` | Start tsumiba (in the tray, `--background`) when you log in. Set it from Settings; that also creates or removes the login entry |
| `renderer` | `"auto"`, `"glow"`, `"wgpu"` | `"auto"` | How the window is drawn (see below). The environment variable `TSUMIBA_RENDERER` overrides it |

**The renderer.** `auto` uses OpenGL (`glow`) everywhere. On **Windows only**, if OpenGL cannot
create the window, tsumiba restarts itself once with the **wgpu** renderer (DirectX 12, and
Microsoft's software rasteriser, WARP, as a last resort). That happens on virtual machines and
remote-desktop sessions that have no graphics driver and only Windows' built-in OpenGL 1.1 stub;
the log then says something like `egui_glow requires opengl 2.0+`. `glow` forces OpenGL
(and shows the error if it fails); `wgpu` forces wgpu. Linux and macOS builds do not contain
wgpu, so `wgpu` there falls back to OpenGL. A forced `glow` never retries, and the retry itself
never loops.

> [!NOTE]
> That fallback is exercised by the GPU-less Windows runner in CI (a job that reports but does
> not fail the build; it passes), not on real virtual machines by the author.

### `[panel]`

| Key | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `show_notes` | boolean | `true` | | Show the notes column |
| `show_files` | boolean | `true` | | Show the files column |
| `width` | number (points) | `1040` | 480–3000 | Width of the panel. Never wider than the monitor |
| `height` | number (points) | `380` | 240–2000 | Height of the panel. Never taller than the monitor |
| `auto_hide` | boolean | `true` | | Hide when the pointer leaves ([Summoning](summoning.md#hiding)) |

If both `show_notes` and `show_files` are `false`, both are turned back on: an empty panel is
never what anyone wants.

### `[trigger]`

| Key | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `hotkey` | string | `"Ctrl+Alt+Space"` | | Global shortcut that toggles the shelf. Empty (`""`) turns it off |
| `edge` | boolean | `true` | | Open when the pointer rests on the top edge |
| `edge_dwell_ms` | integer (ms) | `250` | 0–5000 | How long it must rest. `0` opens at once. Dragging a file ignores it |

**Writing a shortcut.** One or more modifiers, then one key, joined with `+`, in any letter
case: modifiers are `Ctrl`/`Control`, `Alt`/`Option`, `Shift`, `Super`/`Cmd`/`Meta`, and
`CmdOrCtrl` (<kbd>⌘</kbd> on macOS, <kbd>Ctrl</kbd> elsewhere). Keys are letters,
digits, `F1`–`F24`, `Space`, `Enter`, `Tab`, `Up`/`Down`/`Left`/`Right`, `Home`, `End`,
`PageUp`, `PageDown`, `Insert`, `Delete`, `Backspace`, and punctuation such as `-`, `=`,
`,`, `.`, `/`, `;`, `` ` ``. At least one modifier is required, except for function keys.
Examples: `"Ctrl+Alt+Space"`, `"Ctrl+Shift+K"`, `"Alt+F1"`, `"Cmd+Option+Space"`,
`"CmdOrCtrl+Shift+Y"`. On Wayland there are no global shortcuts to register; tsumiba says so when it starts.

### `[storage]`

| Key | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `mode` | `"copy"`, `"link"` | `"copy"` | | What happens to a file you add: keep a private **copy**, or only remember where it is (**link**). Folders are always linked |
| `max_upload_mb` | integer (MiB) | `512` | 1–65536 | The largest file the [HTTP API](api.md) accepts (from the browser extension, for example) |

### `[api]`

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `enabled` | boolean | `true` | Run the local HTTP API used by the command line and the browser extension. Off: the extension cannot connect, and command-line commands work on the files directly |
| `port` | integer | `47821` | Loopback port. `0` lets the system choose a free one. Use `0` or 1024–65535. If the port is taken, tsumiba picks a free one, tells you, and records it for the command line (the extension must then be given the new port) |

## An annotated example

Every value below is the default, so this file changes nothing; delete what you do not need.

```toml
[general]
language = "auto"          # "auto" | "en" | "ja"
theme = "system"           # "system" | "light" | "dark"
launch_at_login = false
renderer = "auto"          # "auto" | "glow" | "wgpu" (wgpu exists on Windows only)

[panel]
show_notes = true
show_files = true
width = 1040.0             # points; 480-3000
height = 380.0             # points; 240-2000
auto_hide = true           # hide when the pointer leaves

[trigger]
hotkey = "Ctrl+Alt+Space"  # "" turns the shortcut off
edge = true                # open when the pointer rests on the top edge
edge_dwell_ms = 250        # 0-5000; dragging a file skips the wait

[storage]
mode = "copy"              # "copy" | "link"
max_upload_mb = 512        # 1-65536

[api]
enabled = true
port = 47821               # 0 = any free port
```

## Environment variables

| Variable | Effect |
| --- | --- |
| `TSUMIBA_HOME` | Keep everything under this folder (`data/` and `config/` inside it) |
| `TSUMIBA_DATA_DIR` | Only the data folder. Wins over `TSUMIBA_HOME` |
| `TSUMIBA_CONFIG_DIR` | Only the configuration folder. Wins over `TSUMIBA_HOME` |
| `TSUMIBA_LOG` | Log verbosity for tsumiba's own messages: `error`, `warn`, `info` (default), `debug`, `trace`, `off` |
| `TSUMIBA_RENDERER` | `auto`, `glow` or `wgpu`: overrides `general.renderer` ([above](#general)) |

The two commands `tsumiba` and `tsumiba quit` must see the same variables, because that is
how they find each other's files.

## Where things are

| | Linux | macOS | Windows |
| --- | --- | --- | --- |
| **Data** (notes, files, trash, log) | `$XDG_DATA_HOME/tsumiba`, usually `~/.local/share/tsumiba` | `~/Library/Application Support/io.github.hjosugi.tsumiba` | `%APPDATA%\hjosugi\tsumiba\data` |
| **Configuration** (`config.toml`, token) | `$XDG_CONFIG_HOME/tsumiba`, usually `~/.config/tsumiba` | the same folder as the data | `%APPDATA%\hjosugi\tsumiba\config` |

What is in them:

| File | Where | What |
| --- | --- | --- |
| `config.toml` | configuration | This page |
| `token` | configuration | The pairing token (readable by you only, on Linux and macOS) |
| `server.json` | configuration | Written while tsumiba runs: its port and process id, so the command line can find it. Removed on exit; a leftover one from a crash is ignored |
| `notes/<id>.md` | data | One file per note |
| `files/<id>/<name>` | data | One folder per stored file |
| `index.json` | data | Pins and file metadata; rebuilt if damaged |
| `trash/` | data | Deleted items, kept 30 days |
| `tsumiba.log`, `tsumiba.log.old` | data | The log; rotated at start-up when it passes 1 MiB |

More about the data folder: [Data and sync](data-and-sync.md).
