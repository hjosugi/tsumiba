# Contributing to tsumiba

Thanks for taking a look. tsumiba is deliberately small: **notes and files, nothing
else** (no clipboard history, no sync service). Changes that keep it that way and keep
it light are the easiest to accept.

## Setting up

With Nix, `nix develop` gives you the Rust toolchain, the libraries, `mdbook`, Node,
and the tools for the headless GUI checks.

Without Nix you need Rust 1.92+ and, on Linux, `pkg-config` and the development files
for `libxcb`, `libxkbcommon`, `libx11` and `wayland` (Debian/Ubuntu:
`libxcb1-dev libxcb-randr0-dev libxkbcommon-dev libx11-dev libwayland-dev`).

## Before you open a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

On Linux, also run the GUI smoke test against a release build:

```sh
cargo build --release
scripts/smoke-gui-linux.sh target/release/tsumiba   # needs xvfb-run and xdotool
```

It starts the real window under a virtual display and exercises summoning (command
line, top edge, drag, shortcut), typing, dismissing and quitting. CI runs it too.

The browser extension has its own tests: `cd extension && npm test` (they talk to a
real `tsumiba` binary: a debug build, or the one named by `TSUMIBA_BIN`; set
`CHROMIUM_BIN` and run `npm run test:browser` for the real-browser tests). The documentation site builds with
`scripts/build-docs-site.sh`.

## How the code is organised

| | |
| --- | --- |
| `crates/tsumiba-core` | Storage, configuration, token, wire types. No UI, no network. Everything here is unit-tested. |
| `crates/tsumiba/src/app` | The window: state machine for showing and hiding, views, theme, fonts. |
| `crates/tsumiba/src/platform` | Anything that touches the OS: pointer, monitors, shortcut, tray, file manager. |
| `crates/tsumiba/src/server.rs` | The local HTTP API. `tests/api.rs` drives it over a real socket. |
| `crates/tsumiba/src/cli.rs` | The command line. `tests/cli.rs` runs the real binary. |
| `extension/` | The browser extension (Manifest V3, no dependencies). |
| `docs-site/` | The documentation site (mdBook, English and Japanese). |

## Conventions

- **User-visible strings** live in `src/i18n.rs` as fields of `Strings`, in English and
  Japanese. A missing translation is a compile error; please write both.
- **Platform code you cannot run** (Windows, macOS) still has to compile and be linted
  in CI. Keep it small, keep it behind `cfg`, and say in the pull request what you did
  and did not verify. Do not mark behaviour as verified on a platform you have not
  tried — the README's table is meant to be trustworthy.
- **Tests belong with the change.** Pure logic (timing rules, parsing, formatting) gets
  unit tests; anything with sockets, processes or the filesystem gets an integration
  test. Security-relevant behaviour (token, Host/Origin checks, file names, size limits)
  is tested in both directions: that the good request works *and* that the bad one is
  refused.
- Commit messages: `type(scope): summary` (`feat`, `fix`, `docs`, `build`, `test`,
  `refactor`), with a body explaining *why* when it is not obvious.

## Reporting bugs

Please include the output of `tsumiba --version` and `tsumiba paths`, your OS and (on
Linux) whether the session is X11 or Wayland, and the end of the log file
(`tsumiba paths` shows where it is; set `TSUMIBA_LOG=debug` for more detail).

## Releasing (maintainers)

1. Move the `Unreleased` entries in `CHANGELOG.md` under the new version heading.
2. Bump `version` in the workspace `Cargo.toml` (the extension takes its version from
   there) and run `cargo check` so `Cargo.lock` follows.
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`. The Release workflow
   builds every platform, checks that the tag matches `Cargo.toml`, and publishes the
   archives, the extension zips and `SHA256SUMS.txt`.
