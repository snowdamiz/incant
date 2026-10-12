#!/bin/sh
# cap.sh <pid> <out.png>: window-only capture (no shadow, no sound) of that PID's main window.
set -eu
id=$(/tmp/incant0044/winctl "$1" window | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
/usr/sbin/screencapture -o -x -l "$id" "$2"
python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(24); print(sys.argv[1].rsplit("/",1)[-1], struct.unpack(">II", d[16:24]))' "$2"
