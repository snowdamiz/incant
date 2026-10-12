#!/bin/sh
# launch.sh <package> <log>: start the 0055 ci-profile window player on one package
# (/tmp/incant0055/pkg/<package>/player) while nobody is using the Mac, record the player's own
# PID and hand the front back. Refuses after the 0055 release marker, without the work.json grant,
# if any 0055 player already runs, or if the player hash differs from pkg/package.json.
set -u
S=/tmp/incant0055
W=$S/winctl
HERE=$(cd "$(dirname "$0")" && pwd)
GRANT=$HERE/../work.json
[ $# -eq 2 ] || { echo "usage: launch.sh <package> <log>"; exit 2; }
PLAYER=$S/pkg/$1/player/incant_game_player
DATA=$S/pkg/$1/player/data
[ -e "$S/released" ] && { echo "0055 native/GPU lease was released; no launch"; exit 2; }
python3 -I -c 'import json,sys; g=json.load(open(sys.argv[1])); sys.exit(0 if g.get("native_gpu_granted") is True and g.get("exclusive_to")=="0055-bidi-wrap-review" else 1)' "$GRANT" \
  || { echo "work.json does not record the 0055 native/GPU grant"; exit 2; }
EXPECTED=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["player_sha256"])' "$S/pkg/package.json")
actual=$(shasum -a 256 "$PLAYER" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "player hash $actual does not match $EXPECTED"; exit 2; }
# Anchored: matches a player's own argv[0], never this script's or a shell's command line.
if pgrep -f "^$S/pkg/" >/dev/null; then echo "a 0055 player is already running"; exit 2; fi
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
mkdir -p "$S/absent-home" "$S/player-tmp"
# exec keeps one PID from the subshell through env and nohup to the player itself.
(cd "$(dirname "$PLAYER")" && exec env -i PATH="$(dirname "$PLAYER")" HOME="$S/absent-home" TMPDIR="$S/player-tmp" \
  /usr/bin/nohup "$PLAYER" --package "$DATA" --window) >"$2" 2>&1 &
NEW=$!
i=0
until ps -o command= -p "$NEW" 2>/dev/null | grep -q "^$PLAYER --package"; do
  sleep 0.1; i=$((i + 1))
  [ $i -ge 50 ] && { echo "pid $NEW is not the player: $(ps -o command= -p "$NEW")"; exit 4; }
done
echo "$NEW" >"$S/main.pid"
echo "$(date -u +%FT%TZ) launched $1 pid $NEW ($(ps -o command= -p "$NEW")) sha256 $actual" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.5; i=$((i + 1))
  [ $i -ge 60 ] && { echo "no window after 30 s"; break; }
done
# The tao window takes the front at launch; return it at once.
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
FRAME=$($W "$NEW" window 2>&1)
# Startup content is 640 x 480 px = 320 x 240 pt; the rest of the frame height is the title bar.
echo "$FRAME" | python3 -I -c 'import json,sys; print(int(round(json.load(sys.stdin)["h"])) - 240)' >"$S/titlebar.pt"
echo "$(date -u +%FT%TZ) $1 pid $NEW window $FRAME titlebar $(cat "$S/titlebar.pt") pt; front returned to $($W frontmost) (waited ${waited}s)" | tee -a "$S/processes.log"
