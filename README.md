# tsumiba

[![CI](https://github.com/hjosugi/tsumiba/actions/workflows/ci.yml/badge.svg)](https://github.com/hjosugi/tsumiba/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/hjosugi/tsumiba)](https://github.com/hjosugi/tsumiba/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Notes and files, one gesture away.** tsumiba (積み場, "a place to pile things")
is a memo and file shelf that slides in from the top edge of your screen — on
**Windows, macOS and Linux**. English | [日本語](README.ja.md)

Move the pointer to the top of the screen (or press a shortcut) and the shelf
drops down: jot a note, drop a file on it, get back to what you were doing. It
goes away on its own when you do, and costs nothing while hidden.

It is the part of [Unclutter](https://unclutterapp.com/) that people reach for
most, rebuilt in Rust for every desktop — **notes and files only**, no clipboard
manager — plus a **browser extension** that sends the page, selection, link or
image you are looking at to the shelf.

- **Notes** — plain Markdown files, saved as you type, searchable, pinnable.
  Opening the shelf puts you in an empty note, ready to type.
- **Files** — drop anything on it. Keep a private copy or just remember where the
  original is. Thumbnails for images; open, reveal, copy or save a copy from the
  context menu. Deleted items stay in a trash for 30 days.
- **Browser extension** — Chrome, Edge, Brave and Firefox. Right-click →
  *Send to tsumiba*, or use the popup for a quick memo.
- **Command line** — `tsumiba note "buy milk"`, `tsumiba add report.pdf`,
  `tsumiba list --json`. Works with or without the app running.
- **Local and private** — no account, no cloud, no telemetry. The only thing it
  listens on is `127.0.0.1`, behind a token.
- **Japanese and English UI**, light and dark themes.

Documentation: **<https://hjosugi.github.io/tsumiba/>**

## Install

Download from the [latest release](https://github.com/hjosugi/tsumiba/releases/latest),
or use an installer script.

### Windows

```powershell
irm https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.ps1 | iex
```

Or unzip `tsumiba-<version>-windows-x86_64.zip` anywhere and run `tsumiba.exe`.
Releases are not code-signed yet, so SmartScreen asks for confirmation the first
time.

### macOS

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

Or open the `.dmg` and drag **Tsumiba** to Applications. The app is ad-hoc
signed, not notarised: on first launch right-click → Open (or run
`xattr -dr com.apple.quarantine /Applications/Tsumiba.app`). One universal build
covers Apple Silicon and Intel.

### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

This installs into `~/.local` (no root). Or take `tsumiba-<version>-linux-<arch>.tar.gz`
and run its `install.sh`. With Nix: `nix run github:hjosugi/tsumiba`.

On Wayland there is no way for an application to watch the pointer or grab a
key, so bind `tsumiba toggle` to a key in your desktop's settings. See
[Platform notes](#platform-notes).

### Build from source

Rust 1.92 or newer.

```sh
cargo build --release          # target/release/tsumiba
cargo test --workspace
```

On Linux you need `pkg-config` and the development files for `libxcb`,
`libxkbcommon`, `libx11` and `wayland`; `nix develop` provides everything,
including a virtual display for the GUI checks.

## Use

| To… | Do |
| --- | --- |
| Open the shelf | Rest the pointer on the **top edge**, press **Ctrl+Alt+Space**, click the tray icon, or run `tsumiba toggle` |
| Open it while dragging | Drag a file to the top edge — it opens at once |
| Write a note | Just type; it is saved half a second after you stop |
| Add files | Drop them on the files side, or **Add files…** |
| Search | **Ctrl/⌘+F**, then type: notes and files are filtered together |
| Keep it open | The pin button in the header |
| Hide it | **Esc**, move the pointer away, or the shortcut again |
| Change anything | The gear button (**Ctrl/⌘+,**), or `config.toml` |

The pointer rests on the top edge for a quarter of a second before the shelf
opens, and only in the middle part of the edge (as wide as the panel), so
reaching for a window's title bar or a browser tab does not trigger it.

## Browser extension

Chrome, Edge, Brave and Firefox (Manifest V3). Download
`tsumiba-extension-chrome-<version>.zip` or `...-firefox-<version>.zip` from the
release, load it, then pair it once with the token from
`tsumiba token` (or **Settings → Browser extension → Copy**). Details, including
what each permission is for, are in [extension/README.md](extension/README.md)
and the [docs](https://hjosugi.github.io/tsumiba/en/browser-extension.html).

## Command line

```sh
tsumiba                       # open the shelf (starts it if needed)
tsumiba toggle                # show/hide; bind this to a key on Wayland
tsumiba note "buy milk"       # or: echo "buy milk" | tsumiba note
tsumiba add report.pdf photos/    # copy files in; --link remembers instead
tsumiba list --json
tsumiba token                 # for pairing the browser extension
tsumiba --help
```

If tsumiba is running, commands go through it so the window stays current; if it
is not, they work on the files directly — so scripts and SSH sessions are fine.

## Where things live

| | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Notes and files | `~/.local/share/tsumiba` | `~/Library/Application Support/io.github.hjosugi.tsumiba` | `%APPDATA%\hjosugi\tsumiba\data` |
| Settings, token | `~/.config/tsumiba` | same as above | `%APPDATA%\hjosugi\tsumiba\config` |

Notes are `notes/<id>.md`, files are `files/<id>/<name>`; both are ordinary files
you can back up, sync or grep. `tsumiba paths` prints the exact locations, and
`TSUMIBA_HOME` moves everything.

## Platform notes

What has and has not been verified, stated plainly:

| | Windows | macOS | Linux (X11) | Linux (Wayland) |
| --- | --- | --- | --- | --- |
| Builds, lints, unit and API/CLI tests | CI | CI | CI | CI |
| The real app starts, runs its window loop, answers the CLI and quits | not yet confirmed | **CI** (`scripts/smoke-gui-macos.sh`, on a GitHub runner) | **CI**, plus summon/dismiss/typing checks under Xvfb (`scripts/smoke-gui-linux.sh`) | not yet confirmed |
| Window position, top-edge trigger, shortcut, drag-to-edge, tray | implemented, **not yet verified on a real machine** | implemented, **not yet verified on a real machine** | **verified under Xvfb** (no real desktop environment) | an ordinary window (implemented, **not yet tried on a real compositor**); no edge trigger or shortcut, Wayland forbids both |

Not implemented yet: dragging a file *out* of the shelf into another app (the
GUI toolkit cannot start a native drag; use **Show in folder**, **Copy file** or
**Save a copy…** meanwhile), code signing, and store/package-manager listings.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports with the output of
`tsumiba paths` and the log file are the most useful kind.

## Prior art and credits

- [Unclutter](https://unclutterapp.com/) by Softwarebay — the idea: a panel at the
  top of the screen for notes and files. macOS only, closed source.
- [yeet](https://github.com/hjosugi/yeet) — a drag-and-drop shelf for Wayland and
  Windows, from which this project's packaging and documentation habits come.
- [Yoink](https://eternalstorms.at/yoink/mac/), [DropPoint](https://github.com/GameGodS3/DropPoint),
  [dragon](https://github.com/mwh/dragon) — other takes on the file shelf.

## License

MIT — see [LICENSE](LICENSE).
