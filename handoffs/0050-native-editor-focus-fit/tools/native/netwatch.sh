#!/bin/sh
# netwatch.sh <pid>: every 0.5 s while the editor lives, list its internet sockets and those of
# its child processes. A sample, not proof: a connection that opens and closes between two
# samples is not seen. Stops nettop when the editor exits.
P=$1
S=/tmp/incant0050
echo "start $(date -u +%FT%TZ) pid $P"
samples=0
while kill -0 "$P" 2>/dev/null; do
  pids=$P
  for child in $(pgrep -P "$P"); do pids="$pids,$child"; done
  out=$(lsof -nP -a -p "$pids" -i 2>/dev/null | tail -n +2)
  [ -n "$out" ] && printf '%s sockets\n%s\n' "$(date -u +%FT%TZ)" "$out"
  [ "$pids" != "$P" ] && echo "$(date -u +%FT%TZ) children $pids"
  samples=$((samples + 1))
  sleep 0.5
done
echo "end $(date -u +%FT%TZ) after $samples samples"
[ -f "$S/nettop.pid" ] && kill "$(cat "$S/nettop.pid")" 2>/dev/null
exit 0
