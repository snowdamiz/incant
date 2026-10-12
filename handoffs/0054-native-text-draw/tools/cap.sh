#!/bin/sh
# cap.sh <pid> <out.png>: window-only capture (no shadow, no sound) of that PID's main window while
# it is frontmost. At most six original captures in this packet; every call counts, kept or not.
set -eu
S=/tmp/incant0054
[ -e "$S/released" ] && { echo "native/GPU ownership was released; no capture"; exit 2; }
used=$(cat "$S/images.log" 2>/dev/null | wc -l | tr -d ' ')
[ "$used" -lt 6 ] || { echo "the six-capture budget is used up; no capture taken"; exit 5; }
[ "$("$S/winctl" frontmost)" = "$1" ] || { echo "pid $1 is not frontmost; no capture taken"; exit 6; }
id=$("$S/winctl" "$1" window | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$(date -u +%FT%TZ) capture $2 window $id $("$S/winctl" "$1" window)" >>"$S/images.log"
/usr/sbin/screencapture -o -x -l "$id" "$2"
python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(24); print(sys.argv[1].rsplit("/",1)[-1], struct.unpack(">II", d[16:24]))' "$2"
