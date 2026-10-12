#!/bin/sh
# launch.sh <log> <fixture.json>: start the granted 0050 editor binary with the project path while
# nobody is using the Mac, record its PID, attach nettop and the socket sampler to it at once, and
# hand the front back. Refuses a binary whose hash differs from the grant.
set -u
S=/tmp/incant0050
W=$S/winctl
BIN=/tmp/incant-native-review-0050-bin-v1/incant_editor
EXPECTED=89f6b0d990cbaf1f275b8b0e7d3c448a543ef7b4199239ec7e57c419c27623fd
NODE_DIR=$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin
HERE=$(cd "$(dirname "$0")" && pwd)
[ $# -eq 2 ] || { echo "usage: launch.sh <log> <fixture.json>"; exit 2; }
actual=$(shasum -a 256 "$BIN" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "binary hash $actual does not match the grant"; exit 2; }
# Anchored: the editor's argv[0] is the binary path. Unanchored, the guard matched its own
# shell command in 0048.
if pgrep -f "^$BIN" >/dev/null; then echo "an editor from this binary is already running"; exit 2; fi
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
PATH="$NODE_DIR:$PATH" nohup "$BIN" "$2" >"$1" 2>&1 &
NEW=$!
# Per-process byte counters from the first second. nettop counts a flow only if it is attached
# while the flow exists, so it starts before the editor can open one.
nettop -P -L 0 -s 1 -x -p "$NEW" -J bytes_in,bytes_out >"$S/nettop-$NEW.csv" 2>&1 &
echo $! >"$S/nettop.pid"
sh "$HERE/netwatch.sh" "$NEW" >"$S/sockets-$NEW.txt" 2>&1 &
echo "$NEW" >"$S/main.pid"
echo "$(date -u +%FT%TZ) launched $NEW sha256 $actual" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.5; i=$((i + 1))
  [ $i -ge 60 ] && { echo "no window after 30 s"; exit 4; }
done
sleep 3
echo "pid $NEW window $($W "$NEW" window) (waited ${waited}s) at $(date -u +%FT%TZ)"
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
