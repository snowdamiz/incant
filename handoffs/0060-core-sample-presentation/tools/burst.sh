#!/bin/sh
# burst.sh "<commands>": one short native review step once nobody is using the Mac (unlocked,
# display awake, 20 s idle), then hand the front back to the app that had it and restore the
# pointer. Exposes $P (player PID), $W (winctl) and $C (cap.sh). winctl input commands abort
# when the player is no longer frontmost, the pointer moved under a person's hand, or the
# session locks; `$W hold N` fails at once on real input (exit 7). A failed step runs nothing
# further. The game ticks only while its window is focused.
# Refuses after release, without a valid lease, or when main.pid is not a running 0060 player.
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
W=$S/winctl
[ -e "$S/main.pid" ] || { echo "no running 0060 player (main.pid absent)"; exit 2; }
P=$(cat "$S/main.pid")
PLAYER=$(cat "$S/current.player")
ps -o command= -p "$P" | grep -q "^$PLAYER --package" || { echo "pid $P is not a running 0060 player"; exit 2; }
waited=0
while [ "$($W locked)" = 1 ] || [ "$($W idle)" -lt 20 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 3600 ] && { echo "no 20 s idle, unlocked interval within 60 min; step skipped"; exit 3; }
  lease_guard
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
rm -f "$S/pointer.last"
echo "$(date -u +%FT%TZ) burst start, run $(cat "$S/current.run"), pid $P, frontmost $PREV, cursor $CURSOR, waited ${waited}s: $1" >>"$S/bursts.log"
P="$P" W="$W" PREV="$PREV" C="$HERE/cap.sh" sh -c "$1"
status=$?
# Release movement keys if the step stopped early (posting needs the player frontmost; a blur
# also clears held keys in the window host).
if [ "$($W frontmost)" = "$P" ]; then for k in 13 0 1 2 49 123 124 125 126; do $W "$P" keyup $k >/dev/null 2>&1; done; fi
if [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  $W activate "$PREV" >/dev/null
fi
if [ $status -ne 7 ]; then $W warp $CURSOR; fi
rm -f "$S/pointer.last"
echo "$(date -u +%FT%TZ) burst end, exit $status, frontmost $($W frontmost), cursor $($W cursor)" >>"$S/bursts.log"
echo "step exit $status (waited ${waited}s)"
exit $status
