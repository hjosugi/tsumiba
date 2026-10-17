# Installation

Every release is on the [releases page](https://github.com/hjosugi/tsumiba/releases/latest),
together with a `SHA256SUMS.txt` file. The installer scripts below download the right
archive for your machine and check it against those sums before installing anything.

> [!NOTE]
> Releases are not code-signed yet. Windows SmartScreen and macOS Gatekeeper will each
> ask you to confirm the first launch; the steps are below.

## Windows

```powershell
irm https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.ps1 | iex
```

This unpacks tsumiba under `%LOCALAPPDATA%\Programs\tsumiba`, adds that folder to your
user `PATH` and creates a Start menu shortcut. No administrator rights are needed.

Prefer to do it by hand? Download `tsumiba-<version>-windows-x86_64.zip`, unzip it
anywhere and run `tsumiba.exe`. Nothing else has to be installed, and nothing is
written outside tsumiba's own folders ([Data and sync](data-and-sync.md)).

SmartScreen says "Windows protected your PC" the first time: choose **More info**,
then **Run anyway**.

## macOS

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

This installs `Tsumiba.app` into `~/Applications` and puts a `tsumiba` command in
`~/.local/bin`. Or take it by hand: open the `.dmg` and drag **Tsumiba** to
Applications, or unzip `tsumiba-<version>-macos-universal.zip`. One universal build
covers Apple Silicon and Intel.

The app is ad-hoc signed but **not notarised**, so Gatekeeper blocks the first launch.
Either right-click the app and choose **Open**, or remove the quarantine flag:

```sh
xattr -dr com.apple.quarantine /Applications/Tsumiba.app
```

tsumiba is an "agent" app: it has no Dock icon and no menu bar of its own. You find it
in the menu bar's status area, and it slides in from the top edge.

## Linux

```sh
curl -fsSL https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.sh | sh
```

This installs into `~/.local` (no root): the `tsumiba` binary, a desktop file and the
icons. Or take `tsumiba-<version>-linux-<arch>.tar.gz` from the release page, unpack it
and run the `install.sh` inside:

```sh
./install.sh                      # into ~/.local
./install.sh --prefix /usr/local  # system-wide (run with sudo)
```

The release page says which architectures were built. Make sure `~/.local/bin` is on
your `PATH`.

### Linux requirements

tsumiba draws with OpenGL and talks to X11 or Wayland, and it loads those libraries when it
starts. A normal desktop already has all of them; a minimal container or server image may
not. If a library is missing, tsumiba says which one and exits. You need:

- OpenGL (`libGL`, `libEGL`): Mesa or your GPU vendor's driver;
- **X11 sessions:** `libX11`, `libxcb` and `libxkbcommon-x11`. The X11 window needs the last one
  (Debian and Ubuntu: `libxkbcommon-x11-0`) at run time; without it tsumiba prints a hint naming
  the package and exits;
- **Wayland sessions:** `libxkbcommon` and `libwayland-client`.

```sh
# Debian, Ubuntu (typical package names)
sudo apt-get install libgl1 libegl1 libxkbcommon0 libxkbcommon-x11-0 libx11-6 libxcb1 libwayland-client0
# Fedora
sudo dnf install mesa-libGL mesa-libEGL libxkbcommon libxkbcommon-x11 libX11 libxcb libwayland-client
# Arch
sudo pacman -S mesa libxkbcommon libxkbcommon-x11 libx11 libxcb wayland
```

On a machine without a GPU driver (a bare VM, a container) Mesa's software renderer works:
set `LIBGL_ALWAYS_SOFTWARE=1`. That is how the author's headless checks run.

> [!NOTE]
> **Wayland:** an application cannot watch the pointer or grab a key there, so the
> top-edge trigger and the global shortcut do not exist. Bind the command
> `tsumiba toggle` to a key in your desktop's keyboard settings. See
> [Summoning the shelf](summoning.md#wayland).

### Nix

```sh
nix run github:hjosugi/tsumiba
```

The repository's flake also provides a development shell: `nix develop`.

## Build from source

You need Rust 1.92 or newer.

```sh
git clone https://github.com/hjosugi/tsumiba
cd tsumiba
cargo build --release          # target/release/tsumiba
cargo test --workspace
```

On Linux you also need `pkg-config` and the development files for `libxcb`, `libxkbcommon`,
`libx11` and `wayland`. On Debian and Ubuntu:

```sh
sudo apt-get install pkg-config libxcb1-dev libxcb-randr0-dev libxkbcommon-dev \
  libx11-dev libwayland-dev
```

`nix develop` provides all of it, plus the tools for the headless GUI checks
([Contributing](contributing.md)).

## Updating and removing

**Updating:** run the installer again, or replace the files by hand. Your notes and
files live elsewhere and are untouched. Quit the old copy first (`tsumiba quit`).

**Removing:** quit tsumiba, delete the program, and, if you want everything gone, the
data and configuration folders (`tsumiba paths` prints them; see
[Data and sync](data-and-sync.md)). If you turned on *Start at login*, turn it off in
Settings first, or remove the autostart entry your system keeps for it.
