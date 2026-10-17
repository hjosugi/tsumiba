# Changelog

All notable changes to tsumiba are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-10-17

The first release.

### Added

- **The shelf**: a window that is created once, hidden, and slides in from the top
  edge of a monitor. While hidden it holds no timers and does no work.
- **Summoning**: rest the pointer on the middle of the top edge (250 ms by
  default), drag something to the top edge (opens at once), a global shortcut
  (`Ctrl+Alt+Space`), the tray icon, or `tsumiba toggle`.
- **Notes**: one Markdown file per note, saved half a second after the last
  keystroke, pinned notes, search, undoable deletion. Opening the shelf starts an
  empty note.
- **Files**: drop files or folders on the shelf; keep a private copy or only
  remember the location (folders are always remembered). Image thumbnails,
  open, show in folder, copy file, copy path, save a copy, rename, pin, remove.
  Removed items stay in a trash for 30 days.
- **Settings screen** and `config.toml`: language (English, 日本語), theme,
  start at login, panel size and which sides to show, shortcut (recorded by
  pressing it), top-edge trigger and delay, copy-or-link, browser API port and
  token.
- **Command line**: `note`, `add`, `list`, `show`, `hide`, `toggle`, `quit`,
  `token`, `paths`, `serve`. Commands use the running app when there is one and
  work on the files directly when there is not.
- **Local HTTP API** on `127.0.0.1` behind a bearer token, with Host and Origin
  checks that refuse DNS-rebinding and cross-site requests.
- **Browser extension** for Chrome, Edge, Brave and Firefox: send the selection,
  a link, an image or the page; popup quick memo; options with pairing.
- **Renderer fallback on Windows**: if OpenGL cannot be used (a virtual machine or
  remote desktop session with no graphics driver), tsumiba restarts itself with
  the wgpu renderer (DirectX 12, with its software rasteriser as a last resort).
  `general.renderer` in `config.toml` (`auto`, `glow`, `wgpu`) or the
  `TSUMIBA_RENDERER` environment variable chooses explicitly.
- Japanese text renders out of the box: a system CJK font is found and used as a
  fallback.
- Packaging: Windows zip, macOS universal `.app` (zip and dmg), Linux tar.gz,
  Nix flake, `install.sh` and `install.ps1` that verify checksums.

### Known limitations

- Dragging a file *out* of the shelf into another application is not supported
  (the GUI toolkit cannot start a native drag).
- On Wayland the shelf is an ordinary window with no top-edge trigger and no
  global shortcut; bind `tsumiba toggle` in the desktop's keyboard settings.
- Windows and macOS builds are produced and tested by CI, but the window, tray
  and shortcut behaviour has not yet been verified on real machines.
- Releases are not code-signed (Windows) or notarised (macOS).

[0.1.0]: https://github.com/hjosugi/tsumiba/releases/tag/v0.1.0
