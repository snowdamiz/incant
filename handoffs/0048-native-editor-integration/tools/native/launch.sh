#!/bin/sh
# launch.sh <log> [fixture.json]: start the granted 0048 editor binary with the project path only
# (or no argument, for the unsaved-project check) while nobody is using the Mac, record its PID,
# and hand the front back. Refuses a binary whose hash differs from the grant.
set -u
W=/tmp/incant0048/winctl
BIN=/tmp/incant-native-review-0048-final-bin-v1/incant_editor
EXPECTED=3d059445c3691b219ff8cc2c6d990645af7f64bbeccee737b4dae4f9753c6603
NODE_DIR=$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin
actual=$(shasum -a 256 "$BIN" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "binary hash $actual does not match the grant"; exit 2; }
if pgrep -f "$BIN" >/dev/null; then echo "an editor from this binary is already running"; exit 2; fi
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
if [ $# -ge 2 ]; then
  PATH="$NODE_DIR:$PATH" nohup "$BIN" "$2" >"$1" 2>&1 &
else
  PATH="$NODE_DIR:$PATH" nohup "$BIN" >"$1" 2>&1 &
fi
NEW=$!
echo "$NEW" > /tmp/incant0048/main.pid
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
