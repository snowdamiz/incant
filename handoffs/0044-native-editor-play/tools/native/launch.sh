#!/bin/sh
# launch.sh <fixture.json> <log>: start the granted editor binary on one fixture (path only, no
# probe flags) while nobody is using the Mac, record its PID, and hand the front back.
set -u
W=/tmp/incant0044/winctl
BIN=/tmp/incant-native-review-0044/incant_editor
EXPECTED=718ea75b9d792015fa2af4d939d4af2e2e412c01c706b046a910075cf70bd1b6
actual=$(shasum -a 256 "$BIN" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "binary hash $actual does not match the grant"; exit 2; }
if pgrep -f "$BIN" >/dev/null; then echo "an editor from this binary is already running"; exit 2; fi
waited=0
while [ "$($W 1 idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 540 ] && { echo "nobody idle for 15 s within 9 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
nohup "$BIN" "$1" >"$2" 2>&1 &
NEW=$!
echo "$NEW" > /tmp/incant0044/main.pid
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.5; i=$((i + 1))
  [ $i -ge 60 ] && { echo "no window after 30 s"; exit 4; }
done
sleep 3
echo "pid $NEW window $($W "$NEW" window) (waited ${waited}s)"
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
