#!/bin/sh
# cap.sh <name> [background]: window-only capture (no shadow, no sound) of the running 0058
# player's main window into runs/<run>/caps/<name>.png, normally while it is frontmost;
# `background` permits a deliberate inactive-window capture. Never a desktop capture. At most
# 12 original captures in this packet; every call that reaches screencapture counts.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
PLAYER=$S/pkg-1/player/incant_game_player
P=$(cat "$S/main.pid")
ps -o command= -p "$P" | grep -q "^$PLAYER --package" || { echo "pid $P is not a running 0058 player"; exit 2; }
OUT=$S/runs/$(cat "$S/current.run")/caps/$1.png
[ "$("$S/winctl" locked)" = 0 ] || { echo "session locked or display asleep; no capture"; exit 8; }
used=$(cat "$S/images.log" 2>/dev/null | wc -l | tr -d ' ')
[ "$used" -lt 12 ] || { echo "the 12-capture budget is used up; no capture taken"; exit 5; }
if [ "${2:-}" != background ]; then
  [ "$("$S/winctl" frontmost)" = "$P" ] || { echo "pid $P is not frontmost; no capture taken"; exit 6; }
fi
[ -e "$OUT" ] && { echo "$OUT exists; captures are never overwritten"; exit 2; }
id=$("$S/winctl" "$P" window | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$(date -u +%FT%TZ) capture $OUT window $id pid $P frontmost $("$S/winctl" frontmost) $("$S/winctl" "$P" window)" >>"$S/images.log"
/usr/sbin/screencapture -o -x -l "$id" "$OUT"
python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(24); print(sys.argv[1].rsplit("/",1)[-1], struct.unpack(">II", d[16:24]))' "$OUT"
