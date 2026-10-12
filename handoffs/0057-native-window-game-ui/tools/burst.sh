#!/bin/sh
# burst.sh "<commands>": one short native review step once nobody is using the Mac (unlocked,
# display awake, 20 s idle), then hand the front back to the app that had it and restore the
# pointer. Exposes $P (player PID), $W (winctl) and $C (cap.sh). winctl input commands abort when
# the player is no longer frontmost, the pointer moved under a person's hand, or the session locks;
# `$W hold N` fails at once on real input. A failed step captures nothing further. The game ticks
# only while its window is focused, so world motion happens inside bursts.
S=$HOME/.codex/incant-evidence/0057
W=$S/winctl
[ -e "$S/released" ] && { echo "0057 native/GPU lease was released; no step"; exit 2; }
P=$(cat $S/main.pid)
ps -o command= -p "$P" | grep -q "^$S/" || { echo "pid $P is not a running 0057 player"; exit 2; }
waited=0
while [ "$($W locked)" = 1 ] || [ "$($W idle)" -lt 20 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 1800 ] && { echo "no 20 s idle, unlocked interval within 30 min; step skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
rm -f "$S/pointer.last"
echo "$(date -u +%FT%TZ) burst start, frontmost $PREV, cursor $CURSOR, waited ${waited}s: $1" >>"$S/bursts.log"
P="$P" W="$W" C="$(cd "$(dirname "$0")" && pwd)/cap.sh" sh -c "$1"
status=$?
if [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  $W activate "$PREV" >/dev/null
fi
# Put the pointer back unless a person moved it during the step.
if [ $status -ne 7 ]; then $W warp $CURSOR; fi
rm -f "$S/pointer.last"
echo "$(date -u +%FT%TZ) burst end, exit $status, frontmost $($W frontmost), cursor $($W cursor)" >>"$S/bursts.log"
echo "step exit $status (waited ${waited}s)"
exit $status
