# Contributing

tsumiba is deliberately small: **notes and files, nothing else.** Changes that keep it that
way, and keep it light, are the easiest to accept. The repository's
[CONTRIBUTING.md](https://github.com/hjosugi/tsumiba/blob/main/CONTRIBUTING.md) is the
canonical guide; this page is the quick version, plus how the headless checks work.

## Set up

With Nix, `nix develop` gives you the Rust toolchain, the libraries, `mdbook`, Node, and the
tools for headless GUI checks (a virtual display, an input injector, a screenshot tool).
Without Nix you need Rust 1.92 or newer and, on Linux, `pkg-config` and the development files
for `libxcb`, `libxkbcommon`, `libx11` and `wayland`.

## Before a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

What the tests cover:

| Where | What |
| --- | --- |
| `crates/tsumiba-core` | The store (adoption, repair, trash, limits), config, ids, file names, the token. No display needed |
| `crates/tsumiba/src` | The edge state machine, monitor geometry, shortcut parsing, slide timing, formatting, translations, colour contrast |
| `crates/tsumiba/tests/api.rs` | A real server on a real socket, driven with hand-written HTTP: authentication, Host and Origin checks, CORS, uploads and their limits, hostile file names |
| `crates/tsumiba/tests/cli.rs` | The real program as a process against a scratch home directory, including against a running `serve` |
| `extension/test` | The browser extension: unit tests; its client and background script against a real `tsumiba serve`; and the built extension loaded into headless Chromium. Run `cd extension && npm test` (needs a debug build of tsumiba, or set `TSUMIBA_BIN`; the browser tests need `chromium` on `PATH`, or `CHROMIUM_BIN`) |

## Testing the window without a screen

The GUI can be driven headlessly on Linux, which is how the "verified" entries in
[Platform support](platforms.md) were produced:

```sh
cargo build --release
scripts/smoke-gui-linux.sh target/release/tsumiba    # CI runs this too
```

It starts the real window under a virtual X server and checks summoning (command line, top
edge, dragging, shortcut), dismissing, typing and quitting. To do your own poking:

```sh
export TSUMIBA_HOME=$(mktemp -d) LIBGL_ALWAYS_SOFTWARE=1 XDG_SESSION_TYPE=x11
xvfb-run -a -s "-screen 0 1440x900x24" bash -c '
  tsumiba --background & sleep 4
  tsumiba show; sleep 1
  xdotool windowfocus "$(xdotool search --onlyvisible --name "^tsumiba$" | head -1)"   # see below
  xdotool mousemove 300 120 click 1 type "typed by a script"
  import -window root shot.png
  tsumiba quit'
```

Two things to know about a virtual display with no window manager:

- **Nothing hands out keyboard focus**, so call `xdotool windowfocus` on the window before
  typing at it (the `windowfocus` line must run *after* the window has been shown, as above).
- **There is no window frame or shadow** in the screenshots; the panel is at `x 200..1240,
  y 0..380` on a 1440×900 screen with the default size.

`LIBGL_ALWAYS_SOFTWARE=1` makes Mesa render in software, which is what a machine with no GPU needs.

Windows and macOS code cannot be run on a Linux machine. CI compiles and lints it, and
smoke-tests it there; do not describe it as verified until someone has used it.

## House rules

- **Every user-visible string needs English and Japanese** (`crates/tsumiba/src/i18n.rs`), and
  the extension has the same rule (`extension/src/lib/i18n.js`).
- A hidden window gets no frames unless something calls for a repaint. Anything that talks to the
  window from another thread must go through `Control::send`.
- Be plain about verification. If it was not run on that system, say *implemented*, not *works*.

## The docs

The site you are reading is built with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
scripts/build-docs-site.sh                    # builds target/docs-site (front page, en/, ja/)
node scripts/check-docs-links.mjs             # broken links and anything loaded from the network
```

The two books (`docs-site/en`, `docs-site/ja`) must have the same pages; the checker fails if
they drift. The theme (`docs-site/theme`) and the screenshots (`docs-site/landing/screenshots`)
are shared, and the build script copies them into each book, so edit the originals. Do not run
`mdbook serve` on a book directly without running the build script once first, or the images
and theme are not there yet.
