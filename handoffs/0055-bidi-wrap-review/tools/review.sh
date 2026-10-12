#!/bin/sh
# review.sh <step> <package>: the 0055 rendered-review steps, each one short burst (burst.sh waits
# for 15 s of idle, yields on input, restores the previous front app and pointer, and refuses after
# release). Window sizes are content points on the 2x display; the title bar height is measured at
# launch (launch.sh) from the startup content of 320 x 240 pt.
#   default  - focus the startup-size window, hold, capture (1:1 layout pixels)
#   large    - resize to the window's maximum content (960 x 540 pt = 1920 x 1080 px), hold, capture
#   restore  - back to the startup size so the closing report reflects a normal window
#   close    - press the window's own close button so the player prints its report and exits
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
S=/tmp/incant0055
OUT=$S/captures
mkdir -p "$OUT"
[ $# -eq 2 ] || { echo "usage: review.sh <step> <package>"; exit 2; }
TB=$(cat "$S/titlebar.pt")
case "$1" in
default) "$HERE/burst.sh" '$W $P raise && $W hold 4 && $W $P window && $C $P '"$OUT/$2"'-1-default.png' ;;
large) "$HERE/burst.sh" '$W $P raise && $W $P frame 120 90 960 '"$((540 + TB))"' && $W hold 5 && $W $P window && $C $P '"$OUT/$2"'-2-large.png' ;;
restore) "$HERE/burst.sh" '$W $P raise && $W $P frame 120 90 320 '"$((240 + TB))"' && $W hold 1 && $W $P window' ;;
close) "$S/winctl" "$(cat "$S/main.pid")" close ;;
*) echo "unknown step $1"; exit 2 ;;
esac
