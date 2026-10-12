#!/bin/sh
# burst.sh "<commands>": run one short review step while nobody is using the Mac, then hand
# the front back to the app that had it and put the pointer back where it was. Uses $P
# (review PID) and winctl's per-event guard.
W=/tmp/incant0044/winctl
P=$(cat /tmp/incant0044/main.pid)
waited=0
while [ "$($W $P idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 540 ] && { echo "nobody idle for 15 s within 9 min; step skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
P="$P" W="$W" sh -c "$1"
status=$?
# Hand the front back, unless a person already took it.
if [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
echo "step exit $status (waited ${waited}s)"
exit $status
