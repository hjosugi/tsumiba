# Changelog

The canonical changelog is
[CHANGELOG.md](https://github.com/hjosugi/tsumiba/blob/main/CHANGELOG.md) in the repository,
in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format; tsumiba follows
[Semantic Versioning](https://semver.org/).

## 0.1.0: the first release (17 October 2026) {#v0-1-0}

**Added**

- **The shelf:** a window created once, hidden, that slides in from the top edge of a monitor.
  While hidden it holds no timers and does no work.
- **Summoning:** rest the pointer on the middle of the top edge (250 ms by default), drag
  something to the top edge (opens at once), a global shortcut (`Ctrl+Alt+Space`), the tray
  icon, or `tsumiba toggle`.
- **Notes:** one Markdown file per note, saved half a second after the last keystroke, pinned
  notes, search, undoable deletion. Opening the shelf starts an empty note.
- **Files:** drop files or folders on the shelf; keep a private copy or only remember the
  location (folders are always remembered). Image thumbnails; open, show in folder, copy file,
  copy path, save a copy, rename, pin, remove. Removed items stay in a trash for 30 days.
- **Settings screen** and `config.toml`: language (English, 日本語), theme, start at login, panel
  size and which sides to show, shortcut (recorded by pressing it), the top-edge trigger and
  its delay, copy or link, the browser API port and token.
- **Command line:** `note`, `add`, `list`, `show`, `hide`, `toggle`, `quit`, `token`, `paths`,
  `serve`. Commands use the running app when there is one and work on the files directly when
  there is not.
- **Local HTTP API** on `127.0.0.1` behind a bearer token, with Host and Origin checks that
  refuse DNS rebinding and cross-site requests.
- **Browser extension** for Chrome, Edge, Brave and Firefox: send the selection, a link, an
  image or the page; popup quick note; options with pairing.
- Japanese text renders out of the box: a system CJK font is found and used as a fallback.
- Packaging: Windows zip, macOS universal `.app` (zip and dmg), Linux tar.gz, a Nix flake, and
  `install.sh` / `install.ps1` that verify checksums.

**Known limitations**

- Dragging a file *out* of the shelf into another application is not supported.
- On Wayland the shelf is an ordinary window with no top-edge trigger and no global shortcut.
- Windows and macOS builds are produced and tested by CI, but the window, tray and shortcut
  behaviour has not been verified on real machines yet.
- Releases are not code-signed (Windows) or notarised (macOS).
