#!/bin/sh
# burst.sh "<commands>": one short review step while nobody is using the Mac, then hand the front
# back to the app that had it and restore the pointer. Exposes $P (player PID), $W (winctl) and $C
# (cap.sh). Steps wait with `$W hold N`, which fails at once on real user input, so the burst
# yields immediately and nothing is captured. The game ticks only while its window is focused.
S=/tmp/incant0055
W=$S/winctl
[ -e "$S/released" ] && { echo "0055 native/GPU lease was released; no step"; exit 2; }
P=$(cat $S/main.pid)
ps -o command= -p "$P" | grep -q "^$S/pkg/" || { echo "pid $P is not a running 0055 player"; exit 2; }
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; step skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
echo "$(date -u +%FT%TZ) burst start, frontmost $PREV, waited ${waited}s: $1" >>"$S/bursts.log"
P="$P" W="$W" C="$(cd "$(dirname "$0")" && pwd)/cap.sh" sh -c "$1"
status=$?
if [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
echo "$(date -u +%FT%TZ) burst end, exit $status, frontmost $($W frontmost)" >>"$S/bursts.log"
echo "step exit $status (waited ${waited}s)"
exit $status
