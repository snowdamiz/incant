#!/bin/sh
# launch.sh <package-dir> <log> [extra player args]: start the 0057 ci-profile window player of
# ~/.codex/incant-evidence/0057/<package-dir>/player once nobody is using the Mac (unlocked, display awake,
# 20 s idle), record the player's own PID and hand the front and pointer back at once.
# Refuses after the 0057 release marker, without the 0057 lease, if any 0057 player already runs,
# or if the player hash differs from <package-dir>/package.json.
set -u
S=$HOME/.codex/incant-evidence/0057
W=$S/winctl
[ $# -ge 2 ] || { echo "usage: launch.sh <package-dir> <log> [extra args]"; exit 2; }
PKG=$1; LOG=$2; shift 2
PLAYER=$S/$PKG/player/incant_game_player
DATA=$S/$PKG/player/data
[ -e "$S/released" ] && { echo "0057 native/GPU lease was released; no launch"; exit 2; }
python3 -I -c 'import json,sys; g=json.load(open(sys.argv[1])); sys.exit(0 if g.get("packet")=="0057-native-window-game-ui" else 1)' "$S/lease.json" \
  || { echo "no 0057 lease marker"; exit 2; }
EXPECTED=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["player_sha256"])' "$S/$PKG/package.json")
actual=$(shasum -a 256 "$PLAYER" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "player hash $actual does not match $EXPECTED"; exit 2; }
# Anchored: matches a player's own argv[0], never this script's or a shell's command line.
if pgrep -f "^$S/" >/dev/null; then echo "a 0057 player is already running"; exit 2; fi
waited=0
while [ "$($W locked)" = 1 ] || [ "$($W idle)" -lt 20 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 1800 ] && { echo "no 20 s idle, unlocked interval within 30 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
mkdir -p "$S/absent-home" "$S/player-tmp"
# exec keeps one PID from the subshell through env and nohup to the player itself.
(cd "$(dirname "$PLAYER")" && exec env -i PATH="$(dirname "$PLAYER")" HOME="$S/absent-home" TMPDIR="$S/player-tmp" \
  /usr/bin/nohup "$PLAYER" --package "$DATA" --window "$@") >"$LOG" 2>&1 &
NEW=$!
i=0
until ps -o command= -p "$NEW" 2>/dev/null | grep -q "^$PLAYER --package"; do
  sleep 0.1; i=$((i + 1))
  [ $i -ge 50 ] && { echo "pid $NEW is not the player: $(ps -o command= -p "$NEW")"; exit 4; }
done
echo "$NEW" >"$S/main.pid"
echo "$(date -u +%FT%TZ) launched $PKG pid $NEW ($(ps -o command= -p "$NEW")) sha256 $actual" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.25; i=$((i + 1))
  [ $i -ge 120 ] && { echo "no window after 30 s"; break; }
done
# The tao window takes the front at launch; return it and the pointer at once.
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  $W activate "$PREV" >/dev/null
  $W warp $CURSOR
fi
FRAME=$($W "$NEW" window 2>&1)
SCALE=$($W "$NEW" scale)
# Startup content is 640 x 480 px, i.e. 480/scale pt high; the rest of the frame is the title bar.
echo "$FRAME" | python3 -I -c 'import json,sys; print(int(round(json.load(sys.stdin)["h"] - 480 / float(sys.argv[1]))))' "$SCALE" >"$S/titlebar.pt"
echo "$(date -u +%FT%TZ) $PKG pid $NEW window $FRAME scale $SCALE titlebar $(cat "$S/titlebar.pt") pt; front returned to $($W frontmost) (waited ${waited}s)" | tee -a "$S/processes.log"
