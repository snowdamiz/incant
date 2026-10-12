#!/bin/sh
# review_run.sh RUN PACKAGE-DIR "<burst steps>" ["<burst steps>" ...]: one supervised
# native-window review run. Waits for IDLE seconds (default 90; the brief's minimum is 20) of
# real user idle, unlocked, before launching and before every burst; launches the package
# with launch.sh (supervised, exit status recorded); runs each step group as one burst
# (burst.sh semantics: $P, $W, $C; `have NAME` is true when this run already holds capture
# NAME). A group that yields to user input (exit 7) is retried after the next idle period,
# at most RETRY times (default 6); a group prefixed `noretry:` (steps whose effect cannot be
# repeated, such as walking) is never retried. Any failure ends the run. The player is then
# closed through AX if still running.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
[ $# -ge 3 ] || { echo "usage: review_run.sh RUN PACKAGE-DIR STEPS [STEPS...]"; exit 2; }
RUN=$1; PKG=$2; shift 2
IDLE=${IDLE:-90}; RETRY=${RETRY:-6}
idle_wait() {
  while [ "$("$S/winctl" locked)" = 1 ] || [ "$("$S/winctl" idle)" -lt "$IDLE" ]; do sleep 5; lease_guard; done
}
idle_wait
"$HERE/launch.sh" "$RUN" "$PKG" >"$S/logs/launch-$RUN.txt" 2>&1 &
LAUNCH=$!
i=0
until grep -q " $RUN pid .* window " "$S/processes.log" 2>/dev/null; do
  sleep 1; i=$((i + 1))
  kill -0 $LAUNCH 2>/dev/null || { echo "launch.sh ended early:"; tail -3 "$S/logs/launch-$RUN.txt"; exit 3; }
  [ $i -ge 120 ] && { echo "no window line after 120 s"; break; }
done
PRE="have() { [ -e '$S/runs/$RUN/caps/'\$1.png ]; }; "
status=0
for steps in "$@"; do
  tries=0; limit=$RETRY
  case "$steps" in noretry:*) steps=${steps#noretry:}; limit=0;; esac
  while :; do
    idle_wait
    "$HERE/burst.sh" "$PRE$steps"; status=$?
    [ $status -eq 0 ] && break
    tries=$((tries + 1))
    echo "burst ended with $status (retry $tries of $limit)"
    { [ $status -ne 7 ] || [ $tries -gt "$limit" ]; } && break
  done
  [ $status -eq 0 ] || break
done
for attempt in 1 2 3 4 5 6; do
  [ -e "$S/main.pid" ] || break
  idle_wait
  "$HERE/burst.sh" '$W $P close' && break
done
wait $LAUNCH
echo "launch.sh: $(tail -1 "$S/logs/launch-$RUN.txt")"
exit $status
