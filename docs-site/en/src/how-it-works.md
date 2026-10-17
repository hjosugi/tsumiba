# How it works

A short tour, for the curious and for contributors. The code is small enough to read; this is
the map.

## Two crates

- **`tsumiba-core`** has no window and no network code. It is the **store** (notes and files on
  disk), `config.toml`, the API token, the record a running instance leaves for the command
  line, and the request and response types of the HTTP API. Everything else builds on it, and
  its tests need no display.
- **`tsumiba`** is the program: the window (egui/eframe), the HTTP server, the command line,
  and the platform layer (pointer, monitors, shortcut, tray, autostart).

## A window that sleeps

The window is created **once, hidden**, and from then on only shown and hidden. While it is
hidden, nothing runs: no timer, no redraw loop. eframe puts a hidden window to sleep until
something asks it to repaint, and the program's helper threads are the only things that do.

Four helpers watch for ways to summon the shelf, each on its own thread:

| Helper | Wakes the window when… |
| --- | --- |
| the global shortcut | the shortcut is pressed |
| the tray icon | you click it or choose a menu item |
| the **pointer watcher** | the pointer has rested on the top edge (it polls about 20 times a second, and only while the shelf is hidden and the setting is on) |
| the HTTP server (four workers) | a request arrives: `show`, `toggle`, or a note from the browser |

They do not touch the window. They send a `Command` (*show*, *hide*, *toggle*, *show on this
monitor*, *quit*, a message for the status line…) through a channel, and then ask for a
repaint, because a command nobody wakes up to read would sit in the queue. The window reads
its commands at the start of every pass. A pass for a hidden window runs the program's own
logic (`App::logic`) but skips drawing, so handling a command costs almost nothing.

Showing is a small state machine: *Hidden → Showing → Shown → Hiding → Hidden*. Showing places
the window just above the top edge of the target monitor and slides it down over 170 ms;
hiding slides it back up and then hides it. On Wayland, where a program cannot place its own
window, it just shows and hides.

Two quirks worth knowing: eframe shows a window after painting its first frame whatever was
asked, so tsumiba starts the window far off-screen and hides it again on the first pass when
it is meant to wait in the tray; and a hidden window's repaint requests are throttled by
eframe to at most one per 100 ms, which is why there is a tenth of a second between a
`tsumiba show` and the window moving.

### The top edge

The pointer watcher is a small, pure state machine fed pointer readings, which is what makes
its timing rules testable without a screen: *touching the top row, in the middle zone, rested
long enough (or the button is down), and not already fired since the pointer last left*. The
pointer is read with each system's own call (`GetCursorPos`, CoreGraphics, `XQueryPointer`),
none of which needs a permission.

### Closing the loop

When it is time to hide, the window hides itself if the pointer has been away for 0.7 s after
having been over it, if focus was lost, and so on ([Summoning the shelf](summoning.md#hiding)).
Notes are saved by the same per-frame logic: a note that has been edited is written half a
second after the last keystroke.

## One process, many doors

Starting `tsumiba` when it is already running does not start another. It looks for the running
one (the port recorded in `server.json`, then the configured port, then a `GET /v1/ping` that
must answer *tsumiba*) and asks it to show itself. The command line works the same way: with a
running app, commands go through its API; without one, they work on the folder directly.

## The store

Notes are files `notes/<id>.md`; stored files live in `files/<id>/<name>`; one small
`index.json` holds what cannot be derived from the folder (pins, sources, links). Ids are
20 hex digits, the creation time followed by random bits, so they sort by age and carry their
own creation date. Everything derivable is derived, and the folder is re-scanned on start and
whenever the shelf opens, which is what makes it safe to sync and lets a damaged index be
rebuilt ([Data and sync](data-and-sync.md)).

Copying a big file must not freeze the panel, so it happens in two steps: *stage* (copy the
bytes into the folder, **without** the store's lock) and *commit* (take the lock for a moment
to add the entry to the index). Writes are atomic: a new file is written and renamed over the
old one.

## Drawing {#drawing}

The window is drawn with egui. By default it uses OpenGL, which is small, quick to start and
available on every desktop with a graphics driver. The exception is a Windows machine whose
only OpenGL is the built-in 1.1 stub (a virtual machine or remote-desktop session with no
driver), where the window cannot be created at all. Windows builds therefore also contain the
**wgpu** renderer (DirectX 12, with a software rasteriser as the last resort) and use it when
OpenGL fails. The window toolkit cannot start a second event loop in the same process, so that
retry is a fresh process started with `TSUMIBA_RENDERER=wgpu`, which the first one waits for, so
that to whoever launched tsumiba it is still a single process. `general.renderer` in
`config.toml`, or `TSUMIBA_RENDERER`, picks a renderer explicitly. Only the GPU-less Windows
runner in CI exercises this fallback; it has not been tried on real virtual machines.

## Text

egui's own fonts have no Japanese. At start-up a background thread scans the system's fonts
for a known CJK family, checks that it really holds Japanese characters, and adds it as a
fallback. Every string in the interface exists in English and Japanese, and a compile-time
struct makes a missing translation a build error, not a blank label.

## Resource use

On the author's test machine, under a virtual display with a software OpenGL renderer, the
process used about 1–3 CPU ticks (10–30 ms) per 20 seconds when idle, hidden or shown, and
roughly 176 MB of resident memory, much of which is the system's CJK font and the software
renderer. Treat these as one data point, not a benchmark; a real GPU changes the memory
picture.
