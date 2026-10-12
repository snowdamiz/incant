#!/bin/sh
# launch.sh <log>: start the exported 0054 window player on its own package while nobody is using
# the Mac, record its PID and hand the front back. Refuses after the recorded 0054 release, without
# the work.json grant, or if the player hash differs from the exported binary recorded in build.txt.
set -u
S=/tmp/incant0054
W=$S/winctl
PLAYER=$S/export/player/incant_game_player
DATA=$S/export/player/data
HERE=$(cd "$(dirname "$0")" && pwd)
GRANT=$HERE/../work.json
[ $# -eq 1 ] || { echo "usage: launch.sh <log>"; exit 2; }
[ -e "$S/released" ] && { echo "native/GPU ownership was released; no launch"; exit 2; }
python3 -I -c 'import json,sys; g=json.load(open(sys.argv[1])); sys.exit(0 if g.get("native_gpu_granted") is True and g.get("exclusive_to")=="0054-native-text-draw" else 1)' "$GRANT" \
  || { echo "work.json does not record the 0054 native/GPU grant"; exit 2; }
EXPECTED=$(cat "$S/player.sha256")
actual=$(shasum -a 256 "$PLAYER" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "player hash $actual does not match $EXPECTED"; exit 2; }
if pgrep -f "^$PLAYER" >/dev/null; then echo "a player from this binary is already running"; exit 2; fi
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
mkdir -p "$S/absent-home"
# exec keeps one PID from the subshell through env and nohup to the player itself.
(cd "$(dirname "$PLAYER")" && exec env -i PATH="$(dirname "$PLAYER")" HOME="$S/absent-home" TMPDIR="$S" \
  /usr/bin/nohup "$PLAYER" --package "$DATA" --window) >"$1" 2>&1 &
NEW=$!
echo "$NEW" >"$S/main.pid"
echo "$(date -u +%FT%TZ) launched $NEW sha256 $actual" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.5; i=$((i + 1))
  [ $i -ge 60 ] && { echo "no window after 30 s"; exit 4; }
done
sleep 2
echo "pid $NEW window $($W "$NEW" window) (waited ${waited}s) at $(date -u +%FT%TZ)"
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
