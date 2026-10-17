#!/usr/bin/env bash
# Start the real window under a virtual X server and check that it can be
# summoned and dismissed the ways a user would: command line, top edge, drag to
# the top edge, global shortcut, and pointer leaving.
#
#   scripts/smoke-gui-linux.sh [path/to/tsumiba]
#
# Needs: xvfb-run, xdotool, and software GL (Mesa llvmpipe).
# This is what "verified on Linux/X11" means in the documentation.
set -euo pipefail

BIN="${1:-${CARGO_TARGET_DIR:-target}/release/tsumiba}"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
[ -x "$BIN" ] || { echo "no executable at $BIN" >&2; exit 2; }

HOME_DIR="$(mktemp -d)"
trap 'rm -rf "$HOME_DIR"' EXIT
export TSUMIBA_HOME="$HOME_DIR"
export TSUMIBA_LOG=debug
export LIBGL_ALWAYS_SOFTWARE=1
export XDG_SESSION_TYPE=x11
unset WAYLAND_DISPLAY

"$BIN" note "seed note for the smoke test" >/dev/null

exec xvfb-run -a -s "-screen 0 1440x900x24" bash -s "$BIN" "$HOME_DIR" <<'INNER'
set -uo pipefail
BIN="$1"; HOME_DIR="$2"
failures=0

visible() { xdotool search --onlyvisible --name '^tsumiba$' 2>/dev/null | head -1; }

# wait_for visible|hidden <timeout seconds>
wait_for() {
  local want="$1" limit="${2:-8}" i
  for ((i = 0; i < limit * 20; i++)); do
    if [ "$want" = visible ] && [ -n "$(visible)" ]; then return 0; fi
    if [ "$want" = hidden ] && [ -z "$(visible)" ]; then return 0; fi
    sleep 0.05
  done
  return 1
}

check() { # check "description" command...
  local what="$1"; shift
  if "$@"; then echo "ok    $what"; else echo "FAIL  $what"; failures=$((failures + 1)); fi
}

"$BIN" --background >"$HOME_DIR/app.out" 2>&1 &
app=$!
trap 'kill $app 2>/dev/null || true' EXIT

# The API is up once the runtime record exists.
for _ in $(seq 1 100); do [ -f "$HOME_DIR/config/server.json" ] && break; sleep 0.1; done
check "starts hidden in the background"            wait_for hidden 5
check "the process is running"                     kill -0 $app

"$BIN" show
check "command line: show"                         wait_for visible
WID="$(visible)"
sleep 0.8   # let the slide-in (about 170 ms) finish before measuring where it came to rest
geometry="$(xdotool getwindowgeometry "$WID" | tr '\n' ' ')"
echo "      window: $geometry"
check "comes to rest against the top edge"         grep -q 'Position: [0-9-]*,0 ' <<<"$geometry"
"$BIN" hide
check "command line: hide"                         wait_for hidden
"$BIN" toggle
check "command line: toggle (open)"                wait_for visible
sleep 0.4
"$BIN" toggle
check "command line: toggle (close)"               wait_for hidden

# The top edge: rest the pointer on it (dwell is 250 ms by default).
xdotool mousemove 700 400; sleep 0.3
xdotool mousemove 700 0
check "pointer resting on the top edge opens it"   wait_for visible
sleep 0.5
xdotool mousemove 700 700
check "pointer leaving closes it"                  wait_for hidden 6

# Brushing past the top must not open it.
xdotool mousemove 700 0; sleep 0.1; xdotool mousemove 700 300; sleep 1
check "brushing past the edge does not open it"    wait_for hidden 1

# Dragging something to the top opens it at once.
xdotool mousemove 300 400; sleep 0.2
xdotool mousedown 1; xdotool mousemove 700 0
check "dragging to the top edge opens it"          wait_for visible
xdotool mouseup 1; sleep 0.3
xdotool mousemove 700 700; wait_for hidden 6 || true

# Global shortcut (default Ctrl+Alt+Space).
xdotool key ctrl+alt+space
check "global shortcut opens it"                   wait_for visible
sleep 0.5
xdotool key ctrl+alt+space
check "global shortcut closes it"                  wait_for hidden

# Typing into a new note and finding it through the API.
"$BIN" show; wait_for visible
WID="$(visible)"; xdotool windowfocus "$WID"; sleep 0.5
xdotool mousemove 600 200 click 1; sleep 0.3
xdotool type --delay 20 "typed by the smoke test"
sleep 1.5
check "a typed note is saved"                      grep -rq "typed by the smoke test" "$HOME_DIR/data/notes"
xdotool key Escape
check "Escape hides the shelf"                     wait_for hidden

"$BIN" quit
for _ in $(seq 1 50); do kill -0 $app 2>/dev/null || break; sleep 0.1; done
check "quit stops the process"                     bash -c "! kill -0 $app 2>/dev/null"
check "the runtime record is removed on exit"      test ! -e "$HOME_DIR/config/server.json"

if [ "$failures" -gt 0 ]; then
  echo "--- log"; grep -v 'XSETTINGS\|0mm' "$HOME_DIR/data/tsumiba.log" | tail -30
  echo "$failures check(s) failed" >&2
  exit 1
fi
echo "all GUI smoke checks passed"
INNER
