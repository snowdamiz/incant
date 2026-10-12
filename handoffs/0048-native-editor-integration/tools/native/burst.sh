#!/bin/sh
# burst.sh "<commands>": run one short review step while nobody is using the Mac, then hand the
# front back to the app that had it and put the pointer back. Uses $P (review PID), $W (winctl)
# and winctl's per-event frontmost guard.
W=/tmp/incant0048/winctl
P=$(cat /tmp/incant0048/main.pid)
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; step skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
P="$P" W="$W" C="$(cd "$(dirname "$0")" && pwd)/cap.sh" sh -c "$1"
status=$?
# Hand the front back, unless a person already took it.
if [ "${KEEP_FRONT:-0}" != 1 ] && [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
echo "step exit $status (waited ${waited}s)"
exit $status
