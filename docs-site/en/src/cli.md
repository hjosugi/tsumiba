# Command line

The same program is the app and the command line. With no command, `tsumiba` opens the
shelf. With one, it does that one thing and exits.

```text
tsumiba [--background] [COMMAND]
```

Commands that change the shelf (`note`, `add`, `show`, `hide`, `toggle`, `quit`) talk to
the **running app** if there is one, so its window stays current and a single process
owns the files. If none is running, `note`, `add` and `list` work **on the files
directly**, so they are safe in scripts, cron jobs and SSH sessions where nothing is
displayed.

## Commands

| Command | What it does |
| --- | --- |
| `tsumiba` or `tsumiba run` | Open the shelf, starting tsumiba if it is not running |
| `tsumiba --background` | Start in the tray without opening the shelf (this is what *Start at login* runs) |
| `tsumiba show` | Show the shelf (starts tsumiba if needed) |
| `tsumiba hide` | Hide the shelf |
| `tsumiba toggle` | Show it if hidden, hide it if shown (starts tsumiba if needed). **Bind this to a key on Wayland.** |
| `tsumiba note [TEXT]… [--pin]` | Add a note |
| `tsumiba add PATH… [--link]` | Add files or folders |
| `tsumiba list [--notes] [--files] [-q TEXT] [--json]` | List notes and files |
| `tsumiba quit` | Quit the running tsumiba |
| `tsumiba token [--regenerate]` | Print the pairing token for the browser extension |
| `tsumiba paths` | Print where tsumiba keeps its files |
| `tsumiba serve [--port N]` | Run the local API without a window |
| `tsumiba --version`, `tsumiba --help` | Version and help |

`hide` and `quit` have nothing to do when tsumiba is not running, and say so. `show`
and `toggle` start it instead, because starting it *is* showing it.

### `note`

```sh
tsumiba note buy milk                       # the words are joined with spaces
tsumiba note $'# Meeting\nagenda: ...'        # bash/zsh: $'...' turns \n into a newline
echo "remember the umbrella" | tsumiba note # no arguments: read standard input
tsumiba note - < draft.md                   # "-" means standard input too
tsumiba note --pin "the thing I am doing now"
```

Prints the new note's id. An empty note is refused.

### `add`

```sh
tsumiba add report.pdf                      # copy it into the shelf
tsumiba add --link big.iso photos/          # remember where they are instead
```

Paths may be relative. For each file it prints a line `ID<TAB>NAME`. If some could not be
added (missing, unreadable) it says which, adds the rest, and exits with failure.
Folders are always linked. With a running app, the app does the copying; the setting in
`storage.mode` decides copy or link unless you pass `--link`.

### `list`

```sh
tsumiba list                                # notes and files, tab-separated
tsumiba list --notes -q "release"           # only notes containing "release"
tsumiba list --files --json | jq -r '.files[].path'
```

Text output is one line per item:

```text
note	01a148d44c055be0439c	* Release checklist
file	01a148d44c5b4e5d2364	screenshot.png	2032
```

That is `note`, id, title (preceded by `* ` when the note is pinned); and `file`, id, name, size in bytes. With
`--json` you get one object, `{"notes": [...], "files": [...]}`, using the same shapes as
the [HTTP API](api.md#data-shapes). `-q` filters the **notes** (it matches their text); it
does not filter files.

### `token`

Prints the 64-hex-digit token the [browser extension](browser-extension.md) needs.
`--regenerate` replaces it, which signs out every paired browser. The running app keeps
the old token until it is restarted (using **Regenerate** in Settings restarts the server
for you).

### `paths`

```text
$ tsumiba paths
data    /home/me/.local/share/tsumiba
config  /home/me/.config/tsumiba/config.toml
token   /home/me/.config/tsumiba/token
log     /home/me/.local/share/tsumiba/tsumiba.log
```

### `serve`

Runs the HTTP API with no window, until `tsumiba quit` (or it is killed). It prints
`listening on 127.0.0.1:PORT`. It refuses to start if tsumiba is already running. This is
what the project's own tests use, and it is handy on a server, but the shelf on a server is
still the same plain files.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | It worked |
| `1` | It failed: the message is on standard error (not running, nothing to save, a file could not be added, …) |
| `2` | The command line itself was wrong (unknown command or flag) |

## Examples

**Bind a key on Wayland.** In GNOME: *Settings → Keyboard → Keyboard Shortcuts → Custom*,
command `tsumiba toggle`. In Sway: `bindsym $mod+space exec tsumiba toggle`. In Hyprland:
`bind = SUPER, space, exec, tsumiba toggle`.

**Capture from anywhere.**

```sh
# the last command's output, as a note
history | tail -20 | tsumiba note

# a screenshot, straight to the shelf (Linux)
grim -g "$(slurp)" /tmp/shot.png && tsumiba add /tmp/shot.png
```

**Over SSH.** Nothing is displayed on the server, and none is needed:

```sh
ssh devbox 'tsumiba note "deploy finished"'
```

The note lands in that machine's shelf. See [Data and sync](data-and-sync.md) if you want
shelves to meet.

**A shell function.**

```sh
n() { tsumiba note "$*"; }     # n call the dentist
```

## Windows

The Windows build is a GUI program, so that no console window flashes up when it starts at
login. When you run it **with arguments from a terminal** it attaches to that terminal, so
`tsumiba --help` and `tsumiba list` print. Because Windows does not make a terminal wait for
GUI programs, `cmd.exe` may show its prompt before the output has finished; PowerShell
usually behaves, and `tsumiba list | more` or `start /wait tsumiba list` make cmd wait. This
path has not been exercised on a real Windows machine yet.

## Environment variables

| Variable | Effect |
| --- | --- |
| `TSUMIBA_HOME` | Put everything under this folder: `data/` and `config/` inside it |
| `TSUMIBA_DATA_DIR` | Only the data folder (wins over `TSUMIBA_HOME`) |
| `TSUMIBA_CONFIG_DIR` | Only the config folder (wins over `TSUMIBA_HOME`) |
| `TSUMIBA_LOG` | Log level for tsumiba's own messages: `error`, `warn`, `info` (default), `debug`, `trace`, `off` |
| `TSUMIBA_RENDERER` | `auto`, `glow` or `wgpu`: how the window is drawn; overrides `general.renderer` ([Configuration](configuration.md#general)) |

Using `TSUMIBA_HOME` is also how you run a second, independent shelf, or a throwaway one
for testing.
