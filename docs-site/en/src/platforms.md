# Platform support

What works where, and, just as important, how sure anyone is. The legend:

- <span class="tag">verified</span> the author ran it end to end (on Linux/X11, under a virtual display with a
  scripted pointer and keyboard).
- <span class="tag">CI</span> compiled, linted and covered by automated tests on every push, on that operating
  system. Logic and storage, not a window on a screen.
- <span class="tag">implemented</span> the code is there; nobody has run it on that system yet.
- <span class="tag">not possible</span> the operating system forbids it.

## The matrix

| | Linux (X11) | Linux (Wayland) | Windows | macOS |
| --- | --- | --- | --- | --- |
| Builds, lints, unit tests | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> |
| Storage, HTTP API and command-line tests (they run the real program) | <span class="tag">verified</span> <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> | <span class="tag">CI</span> |
| The real program starts, its event loop and API come up, `show` / `hide` / `quit` are accepted | <span class="tag">verified</span> | <span class="tag">CI</span> | <span class="tag">CI</span> smoke test (informational) | <span class="tag">CI</span> smoke test (informational) |
| The window: starts hidden, shows and hides, slides in | <span class="tag">verified</span> | ordinary window, no slide; <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Window creation with no GPU (software rendering) | <span class="tag">verified</span> (Mesa software renderer, as in every check above) | <span class="tag">implemented</span> | <span class="tag">CI</span> GPU-less runner, informational; wgpu fallback, not tried on real VMs | <span class="tag">implemented</span> |
| Notes: write, auto-save, switch, search | <span class="tag">verified</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Notes: pin, delete + undo | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Files: tiles, thumbnails; the settings screen | <span class="tag">verified</span> (looked at) | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Files: right-click menu, rename, remove + undo | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Top-edge trigger (rest, brush past, drag) | <span class="tag">verified</span> | <span class="tag">not possible</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Global shortcut | <span class="tag">verified</span> | <span class="tag">not possible</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| `tsumiba show / hide / toggle` | <span class="tag">verified</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Tray icon | <span class="tag">implemented</span> (needs a status-notifier host; GNOME: AppIndicator extension) | <span class="tag">implemented</span> (same) | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Dropping files from other programs onto the shelf | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Add files…, Save a copy… (native dialogs) | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Open, Show in folder, Copy file | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Japanese text (system CJK font fallback) | <span class="tag">verified</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Japanese text input (IME) | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Start at login | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> | <span class="tag">implemented</span> |
| Dragging a file *out* of the shelf | <span class="tag">not implemented</span> | <span class="tag">not implemented</span> | <span class="tag">not implemented</span> | <span class="tag">not implemented</span> |
| Code signing and notarisation | none | none | none | ad-hoc signed only |

The [browser extension](browser-extension.md): its logic, its conversation with a real
tsumiba server, and the built extension loaded into **headless Chromium** (service worker,
menus, shortcut, popup, options) are covered by automated tests, and the Firefox build is
checked with `web-ext lint`. It has **never been run in Firefox, Edge or Brave**, so those are
<span class="tag">implemented</span> only.

## Reading the table

**Linux/X11 is the reference.** Everything marked *verified* was exercised there, under a
virtual display (Xvfb) with a scripted keyboard and pointer: the window staying hidden at
start-up, appearing from `tsumiba show`, `toggle` and the shortcut, the edge trigger
(including that brushing past does not open it and that dragging opens it at once), hiding
when the pointer leaves, typing and saving notes, searching, the file tiles and thumbnails, the settings screen,
Japanese text, and the whole command line against a running app. The repository contains the script
that does the smoke-test part (`scripts/smoke-gui-linux.sh`), which CI runs.

**Windows and macOS are built and tested, but not yet seen working.** The same program is
compiled for them, the storage layer, HTTP API and command line are tested there, a smoke
test in CI starts the real program on each (and reports, without failing the build, whether
the window could be created on a runner with no GPU), and the
platform-specific parts (reading the pointer, the tray, the shortcut, the console
attachment) are written against each system's own interfaces. What nobody has done is sit
at a real Windows or macOS machine and use the shelf. Expect rough edges there, and please
[report them](https://github.com/hjosugi/tsumiba/issues).

**Wayland is a deliberate limitation, not a bug.** Wayland's design stops an application from
reading the pointer outside its windows, grabbing a key globally, or positioning and raising
its own window. So the edge trigger and the shortcut cannot exist there. The shelf is an
ordinary window and you bind `tsumiba toggle` to a key
([Summoning the shelf](summoning.md#wayland)).

**Drag-out is not implemented anywhere**, for a reason that lives in the window toolkit
([Files](files.md#dragging-files-out)).

## Not planned

By design, tsumiba does not have a clipboard history, cloud sync, or accounts. The scope
is notes and files.
