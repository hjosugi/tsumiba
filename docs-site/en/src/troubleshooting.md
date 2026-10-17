# Troubleshooting and FAQ

First, two things that make any report or fix easier:

```sh
tsumiba paths                    # where the config, data and log are
TSUMIBA_LOG=debug tsumiba        # run in a terminal with detailed logging
```

The **log file** is `tsumiba.log` in the data folder (`tsumiba paths` prints it; the
previous one is `tsumiba.log.old`). Start-up problems end up there even on Windows, where
the program has no console to print to.

## Starting

### Nothing appears when I start it {#nothing-appears}

Starting tsumiba opens the shelf, so if nothing does, look at the terminal output (run
`tsumiba` from a terminal) and the log. Common causes:

- **A system library is missing** (Linux). tsumiba loads OpenGL, `libxkbcommon`, the X11
  keyboard library `libxkbcommon-x11` (X11 sessions), and the X11 or Wayland libraries when it
  starts, and says which one is missing. The commonest case is *tsumiba needs the X11 keyboard
  library*: install `libxkbcommon-x11-0` (Debian, Ubuntu) or `libxkbcommon-x11` (Fedora, Arch).
  See [Linux requirements](installation.md#linux-requirements).
- **"The window could not be created"** with an OpenGL message. tsumiba needs OpenGL 2.0 or
  newer (3.x is better). On Linux machines without a GPU driver, `LIBGL_ALWAYS_SOFTWARE=1` uses
  Mesa's software renderer. On Windows, see [the window cannot be created](#windows-opengl) below.
- **It is already running.** Starting it a second time asks the running one to show itself.
  If the shelf does not appear, try `tsumiba toggle`, or `tsumiba quit` and start again.

### I see "port 47821 is taken" {#port-taken}

Another program holds the port. tsumiba picks a free one and tells you; the command line
finds it by itself. The **browser extension** needs the new port in its options (the new
number is shown in **Settings → Browser extension**). To avoid it, set `api.port` to a
free port, or `0` to always let the system choose.

## Summoning

### The edge trigger opens it while I am just reaching for browser tabs {#edge-opens-by-accident}

Reaching a tab strip at the top of the screen with the pointer resting there for a moment
can look like a summons. The trigger is already cautious (the **middle part** of the edge
only, as wide as the panel, and a **250 ms** rest). Options:

- raise the delay: **Settings → Summon → Delay**, or `trigger.edge_dwell_ms = 500`;
- make the panel narrower, so the active stretch is smaller;
- turn the edge off (`trigger.edge = false`) and use the shortcut only.

Note that **dragging** has no delay by design, so dragging something across the very top of
the screen can open it.

### The edge trigger does nothing {#edge-does-nothing}

- On **Wayland** there is no edge trigger ([Summoning the shelf](summoning.md#wayland)).
- Is it on? **Settings → Summon → Reveal at the top edge**.
- The pointer has to be on the very top row of the screen, in the middle part. With two
  monitors, each has its own edge. If monitors are stacked vertically, the pointer moves
  straight through the boundary instead of stopping at the top row of the lower one.
- On macOS the shelf opens just below the menu bar, but the trigger is the very top.

### The shortcut does nothing, or Settings says it is not available {#shortcut-does-nothing}

- On **Wayland** global shortcuts are not available. Bind `tsumiba toggle` in your desktop's
  keyboard settings ([Command line](cli.md#examples)).
- Another program may already own <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Space</kbd>. tsumiba
  says so when you try to set it and keeps the old one. Pick another combination.
- Input-method switchers sometimes use the same keys. Try <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Space</kbd>
  or a function key.

### There is no tray icon {#no-tray-icon}

- **GNOME** does not show status icons without the *AppIndicator and KStatusNotifierItem
  Support* extension. Other desktops with a status-notifier tray work out of the box.
- The shelf is still reachable through the shortcut, the edge and `tsumiba toggle`.

### It opens, but my typing goes nowhere {#no-keyboard-focus}

The shelf is a window like any other and needs keyboard focus to receive typing. Operating
systems sometimes refuse to hand focus to a window that appeared because the *pointer*
touched the edge rather than because you pressed a key. Click once inside the shelf. If it
happens every time, summon it with the shortcut instead. (This has not been reproduced on
a real Windows or macOS machine.)

## Text and appearance

### Japanese text shows as empty boxes {#japanese-boxes}

tsumiba does not carry a Japanese font. On start-up it looks for one your system already
has (Hiragino Sans on macOS, Yu Gothic UI or Meiryo on Windows, Noto Sans CJK JP, IPAexGothic
and others on Linux), checks that it really contains the characters, and uses it as a fallback. It
does this in the background, so the first moments may show boxes. If they stay, no suitable
font was found: install one (on Debian or Ubuntu, `fonts-noto-cjk`) and restart. The log says
which font was used (`using … for Japanese text`) or that none was found.

### I cannot type Japanese {#japanese-input}

Input methods work through the windowing system. If conversion does not appear, check that
your input method is running and offers itself to X11 or Wayland applications. This has not
been verified with every IME.

### It is too small, too big, or on the wrong monitor {#size-and-monitor}

Change the size in **Settings → Panel** (live), or `panel.width` and `panel.height`. It opens
on the monitor under the pointer. On screens with different scaling, placement is best-effort.

## Files and notes

### A tile says "missing" {#tile-missing}

That file was added as a **link**, and the original has since been moved or deleted. Remove
the tile, or put the file back where it was. Links never copy the file; see
[Files](files.md#copy-or-link).

### I deleted something {#deleted-something}

Use **Undo** in the message at the bottom within six seconds. Later, it is still in the
trash folder for 30 days: [Data and sync](data-and-sync.md#the-trash).

### My notes disappeared after a sync {#sync-lost-notes}

Check for conflict copies in `notes/`, and read [Syncing](data-and-sync.md#syncing). If
`index.json` was damaged, the shelf rebuilt it, and only pins, sources and links are lost.

### A file will not upload from the browser: "too large" {#upload-too-large}

The API limit is `storage.max_upload_mb`, 512 MiB by default.

## Operating systems

### Windows: "Windows protected your PC" {#smartscreen}

The program is not code-signed yet, so SmartScreen does not recognise it. Choose **More info**,
then **Run anyway**. You can verify the download against `SHA256SUMS.txt` first.

### Windows: the window cannot be created (virtual machine, remote desktop) {#windows-opengl}

A Windows machine with no graphics driver (many virtual machines, some remote-desktop sessions)
only has Windows' built-in OpenGL 1.1 stub, which tsumiba's default renderer cannot use; the
log says something like `egui_glow requires opengl 2.0+`. On Windows, tsumiba handles this by
itself: with the default setting (`general.renderer = "auto"`), if OpenGL cannot create the
window it **restarts itself once with the wgpu renderer** (DirectX 12, falling back to
Microsoft's software rasteriser, WARP). You do not need to do anything; there is one extra
start-up attempt, so it takes a moment longer.

To choose explicitly, set `general.renderer` to `"wgpu"` (or `"glow"`) in `config.toml`, or set
the environment variable `TSUMIBA_RENDERER` (`wgpu`, `glow`, `auto`), which wins over the
file. Linux and macOS builds contain only OpenGL.

> [!NOTE]
> The fallback is exercised by the GPU-less Windows runner in CI (a job that reports but does not
> fail the build; it passes), not on real virtual machines by the author. If it does not work for you, please
> [report it](https://github.com/hjosugi/tsumiba/issues) with the log file.

### macOS: "Tsumiba is damaged" or "cannot be opened" {#gatekeeper}

The app is ad-hoc signed but not notarised, so the quarantine flag blocks it. Right-click →
**Open**, or:

```sh
xattr -dr com.apple.quarantine /Applications/Tsumiba.app
```

### Linux: it works in X11 but not Wayland (or the reverse) {#x11-wayland}

On Wayland the shelf is an ordinary window with no edge trigger and no shortcut. If you want
the X11 behaviour under a Wayland desktop, `WINIT_UNIX_BACKEND=x11 tsumiba` runs it
through XWayland (untested, and the pointer and keys are only seen while they are over X11
windows).

## FAQ

**Why no clipboard history?** Scope. Unclutter has one; plenty of tools do clipboard history
well. tsumiba does notes and files and tries to be good at those.

**Why can I not drag a file out?** The window toolkit cannot start a native drag. See
[Files](files.md#dragging-files-out) for the reasons and the workarounds.

**Does it sync between my computers?** There is no built-in sync; the data is plain files,
so any file-sync tool works ([Data and sync](data-and-sync.md#syncing)).

**Can I run it from a USB stick?** Yes: set `TSUMIBA_HOME` to a folder on the stick before
starting it. Nothing is written elsewhere (except the login entry, if you ask for one).

**Does it phone home or check for updates?** No ([Privacy and security](privacy-and-security.md)).

**Can I run two shelves?** Yes, with different `TSUMIBA_HOME` values. They use different
folders, and (if both run) different ports: set `api.port` differently, or `0`.
