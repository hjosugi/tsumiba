#!/usr/bin/env bash
# Start the real tsumiba on macOS and check that the app, its event loop and the
# local API come up and answer. Informational in CI (see the workflow).
set -uo pipefail

BIN="${1:-target/release/tsumiba}"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
HOME_DIR="$(mktemp -d)"
export TSUMIBA_HOME="$HOME_DIR" TSUMIBA_LOG=debug
failures=0
check() { local what="$1"; shift; if "$@"; then echo "ok    $what"; else echo "FAIL  $what"; failures=$((failures + 1)); fi; }

"$BIN" --background >"$HOME_DIR/app.out" 2>&1 &
app=$!
sleep 6
check "the app is still running after start-up" kill -0 $app
if ! kill -0 $app 2>/dev/null; then
  cat "$HOME_DIR/app.out"; tail -40 "$HOME_DIR/data/tsumiba.log" 2>/dev/null; exit 1
fi

check "a note goes through the running app"  bash -c "'$BIN' note 'written while the app runs' | grep -Eq '^[0-9a-f]{20}\$'"
check "the note is listed"                  bash -c "'$BIN' list | grep -q 'written while the app runs'"
check "show is accepted"                    "$BIN" show
sleep 2
check "the app survived showing its window" kill -0 $app
check "hide is accepted"                    "$BIN" hide
check "quit is accepted"                    "$BIN" quit
for _ in $(seq 1 100); do kill -0 $app 2>/dev/null || break; sleep 0.1; done
check "the process exits"                   bash -c "! kill -0 $app 2>/dev/null"

echo "--- tsumiba.log"; tail -40 "$HOME_DIR/data/tsumiba.log" 2>/dev/null
rm -rf "$HOME_DIR"
if [ "$failures" -gt 0 ]; then echo "$failures check(s) failed" >&2; exit 1; fi
echo "all macOS GUI smoke checks passed"
