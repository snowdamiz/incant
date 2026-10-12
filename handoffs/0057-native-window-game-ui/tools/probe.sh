#!/bin/sh
# probe.sh <package-dir> <log>: the window player's own --lifecycle-probe on an ordinary package
# (the package's startup files passed as the explicit --window arguments; --package does not
# forward the probe flag). The probe requests a resize, a 250 ms minimize/restore and a surface
# recreation, stops at tick 40 and prints its validated report. It takes the front itself, so this
# runs only in a 20 s idle, unlocked interval and then returns the front app and pointer.
set -u
S=$HOME/.codex/incant-evidence/0057
W=$S/winctl
[ $# -eq 2 ] || { echo "usage: probe.sh <package-dir> <log>"; exit 2; }
PLAYER=$S/$1/player/incant_game_player
DATA=$S/$1/player/data
[ -e "$S/released" ] && { echo "0057 native/GPU lease was released; no probe"; exit 2; }
EXPECTED=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["player_sha256"])' "$S/$1/package.json")
[ "$(shasum -a 256 "$PLAYER" | cut -d' ' -f1)" = "$EXPECTED" ] || { echo "player hash mismatch"; exit 2; }
if pgrep -f "^$S/" >/dev/null; then echo "a 0057 player is already running"; exit 2; fi
CAMERA=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["camera"])' "$DATA/package.json")
waited=0
while [ "$($W locked)" = 1 ] || [ "$($W idle)" -lt 20 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 1800 ] && { echo "no 20 s idle, unlocked interval within 30 min; probe skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
(cd "$(dirname "$PLAYER")" && exec env -i PATH="$(dirname "$PLAYER")" HOME="$S/absent-home" TMPDIR="$S/player-tmp" \
  /usr/bin/nohup "$PLAYER" --window "$DATA/world.scene" "$DATA/audio" "$DATA/clips.bin" "$DATA/input-actions.json" \
  "$DATA/models" "$DATA/textures" "$CAMERA" --font-cache "$DATA/fonts" --lifecycle-probe) >"$2" 2>&1 &
NEW=$!
echo "$(date -u +%FT%TZ) lifecycle probe pid $NEW ($(ps -o command= -p "$NEW" | cut -c1-120)...)" >>"$S/processes.log"
i=0
while ps -p "$NEW" >/dev/null 2>&1; do
  sleep 0.25; i=$((i + 1))
  [ $i -ge 160 ] && { echo "probe still running after 40 s"; break; }
done
wait "$NEW"; status=$?
[ "$($W frontmost)" != "$PREV" ] && $W activate "$PREV" >/dev/null
$W warp $CURSOR
echo "$(date -u +%FT%TZ) lifecycle probe pid $NEW exit $status; front returned to $($W frontmost) (waited ${waited}s)" | tee -a "$S/processes.log"
exit $status
