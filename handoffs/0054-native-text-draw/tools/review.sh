#!/bin/sh
# review.sh <step>: the 0054 rendered-review steps, each one short burst (burst.sh waits for 15 s
# of idle, restores the previous front app and pointer, and refuses after release).
#   default  - focus the launched window at its startup size, let the game run, capture
#   large    - resize to the window's maximum (960 x 540 pt content), let it run, capture
#   minimum  - resize to the smallest frame the window accepts, let it run, capture
#   restore  - back to the startup size so the closing report reflects a normal window
#   close    - press the window's own close button so the player prints its report and exits
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=/tmp/incant0054/captures
mkdir -p "$OUT"
case "$1" in
default) "$HERE/burst.sh" '$W $P raise && sleep 5 && $W $P window && $C $P '"$OUT"'/1-default.png' ;;
large) "$HERE/burst.sh" '$W $P raise && $W $P frame 80 60 960 568 && sleep 4 && $W $P window && $C $P '"$OUT"'/2-large.png' ;;
minimum) "$HERE/burst.sh" '$W $P raise && $W $P frame 80 60 8 36 && sleep 3 && $W $P window && $C $P '"$OUT"'/3-minimum.png' ;;
restore) "$HERE/burst.sh" '$W $P raise && $W $P frame 80 60 320 268 && sleep 2 && $W $P window' ;;
close) /tmp/incant0054/winctl "$(cat /tmp/incant0054/main.pid)" close ;;
*) echo "unknown step $1"; exit 2 ;;
esac
