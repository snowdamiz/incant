#!/bin/sh
# cap.sh <pid> <out.png>: window-only capture (no shadow, no sound) of that PID's main window.
# The packet allows at most six captures, crops included; images.log counts both.
set -eu
S=/tmp/incant0050
used=$(cat "$S/images.log" 2>/dev/null | wc -l | tr -d ' ')
[ "$used" -lt 6 ] || { echo "the six-image budget is used up; no capture taken"; exit 5; }
id=$("$S/winctl" "$1" window | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
/usr/sbin/screencapture -o -x -l "$id" "$2"
echo "$(date -u +%FT%TZ) capture $2" >>"$S/images.log"
python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(24); print(sys.argv[1].rsplit("/",1)[-1], struct.unpack(">II", d[16:24]))' "$2"
