#!/bin/sh
# cap.sh <pid> <out.png> [background]: window-only capture (no shadow, no sound) of that PID's
# main window, normally while it is frontmost; `background` permits the deliberate blurred-window
# capture. At most 40 original captures in this packet; every call counts, kept or not.
set -eu
S=$HOME/.codex/incant-evidence/0057
[ -e "$S/released" ] && { echo "0057 native/GPU lease was released; no capture"; exit 2; }
[ "$("$S/winctl" locked)" = 0 ] || { echo "session locked or display asleep; no capture"; exit 8; }
used=$(cat "$S/images.log" 2>/dev/null | wc -l | tr -d ' ')
[ "$used" -lt 40 ] || { echo "the 40-capture budget is used up; no capture taken"; exit 5; }
if [ "${3:-}" != background ]; then
  [ "$("$S/winctl" frontmost)" = "$1" ] || { echo "pid $1 is not frontmost; no capture taken"; exit 6; }
fi
[ -e "$2" ] && { echo "$2 exists; captures are never overwritten"; exit 2; }
id=$("$S/winctl" "$1" window | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$(date -u +%FT%TZ) capture $2 window $id frontmost $("$S/winctl" frontmost) $("$S/winctl" "$1" window)" >>"$S/images.log"
/usr/sbin/screencapture -o -x -l "$id" "$2"
python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(24); print(sys.argv[1].rsplit("/",1)[-1], struct.unpack(">II", d[16:24]))' "$2"
