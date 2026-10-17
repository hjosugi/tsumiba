# tsumiba — notes for coding agents

A cross-platform memo and file shelf (Rust, egui/eframe) plus a browser extension and
a documentation site. Scope is deliberately **notes and files only**.

## Commands

```sh
nix develop                                   # toolchain, libs, mdbook, node, xvfb
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release && scripts/smoke-gui-linux.sh target/release/tsumiba
(cd extension && npm test)                    # needs a debug build of tsumiba (TSUMIBA_BIN)
scripts/build-docs-site.sh                    # docs site -> target/docs-site
```

## Map

- `crates/tsumiba-core` — store (notes `<id>.md`, files `<id>/<name>`, rebuildable
  `index.json`, 30-day trash), config, token, wire types. No UI, no network.
- `crates/tsumiba/src/app` — the window. `mod.rs` owns the show/hide state machine and
  applies `Action`s that the views (`notes.rs`, `files.rs`, `settings.rs`, `panel.rs`)
  only describe.
- `crates/tsumiba/src/platform` — pointer, monitors, edge detector (pure state
  machine), hotkey, tray, shell integration, autostart.
- `crates/tsumiba/src/server.rs` — local HTTP API; Host + Origin + bearer-token checks.
- `src/control.rs` — how helper threads command the window (`Command` + repaint wake).

## Things that are easy to get wrong

- A hidden window gets no frames unless something calls `request_repaint`; anything
  that talks to the window from another thread must go through `Control::send`.
- eframe shows the window after its first frame regardless of `with_visible(false)`;
  `App::logic` hides it again on the first pass.
- Windows and macOS code cannot be run on the Linux dev machine. CI compiles and lints
  it; do not claim it is verified.
- Every user-visible string needs English *and* Japanese in `src/i18n.rs`.
- `CARGO_TARGET_DIR` may be set globally; the debug binary is not at `./target`.
