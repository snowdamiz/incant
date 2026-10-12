#!/bin/sh
# burst.sh "<commands>": run one short review step while nobody is using the Mac, then hand the
# front back to the app that had it and put the pointer back. Uses $P (review PID), $W (winctl),
# $C (cap.sh) and winctl's per-event frontmost guard. The front app and focused element at the
# start of each burst are logged. While the editor is in the background its focused element reads
# null, so attributing a change between bursts relies on the first reads after it comes forward.
# Copied from 0050 with 0052's scratch directory; refuses to run after the recorded release.
# Native Play holds (Suspended) while the editor is not frontmost, so a running session only
# advances inside a burst.
S=/tmp/incant0052
W=$S/winctl
[ -e "$S/released" ] && { echo "native/GPU ownership was released; no step"; exit 2; }
P=$(cat $S/main.pid)
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; step skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
{
  echo "$(date -u +%FT%TZ) burst start, frontmost $PREV"
  $W "$P" ax focused
} >>"$S/bursts.log" 2>&1
P="$P" W="$W" C="$(cd "$(dirname "$0")" && pwd)/cap.sh" sh -c "$1"
status=$?
# Hand the front back, unless a person already took it.
if [ "$($W frontmost)" = "$P" ] && [ "$PREV" != "$P" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
echo "$(date -u +%FT%TZ) burst end, exit $status, frontmost $($W frontmost)" >>"$S/bursts.log"
echo "step exit $status (waited ${waited}s)"
exit $status
