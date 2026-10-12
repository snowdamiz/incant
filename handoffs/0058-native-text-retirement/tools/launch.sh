#!/bin/sh
# launch.sh <run>: start the ordinary 0058 ci-profile window player (pkg-1, `--package data
# --window`) once nobody is using the Mac (unlocked, display awake, 20 s idle), hand the front
# app and pointer back at once, then SUPERVISE it: wait for its exit and record the real exit
# status (128+N for signal N) in runs/<run>/exit.json and the process ledger, with any new macOS
# crash report names. Run it as a background task. The player's stdout (its final JSON report)
# and stderr go to runs/<run>/player.log.
# Refuses after release, without a valid lease, if a 0058 player already runs, if the run
# directory exists, or if the player hash differs from pkg-1/package.json.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
[ $# -eq 1 ] || { echo "usage: launch.sh <run>"; exit 2; }
RUN=$1
W=$S/winctl
PKG=$S/pkg-1
PLAYER=$PKG/player/incant_game_player
DATA=$PKG/player/data
OUT=$S/runs/$RUN
[ -e "$OUT" ] && { echo "$OUT exists; runs are never reused"; exit 2; }
EXPECTED=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["player_sha256"])' "$PKG/package.json")
actual=$(shasum -a 256 "$PLAYER" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "player hash $actual does not match $EXPECTED"; exit 2; }
# Anchored on the frozen player path: matches a player's own argv[0], never a shell or winctl.
if pgrep -f "^$PLAYER" >/dev/null; then echo "a 0058 player is already running"; exit 2; fi
waited=0
while [ "$($W locked)" = 1 ] || [ "$($W idle)" -lt 20 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 3600 ] && { echo "no 20 s idle, unlocked interval within 60 min; launch skipped"; exit 3; }
  lease_guard
done
mkdir -p "$OUT/caps" "$S/absent-home" "$S/player-tmp"
PREV=$($W frontmost)
CURSOR=$($W cursor)
touch "$OUT/launched.marker"
# exec keeps one PID from the subshell through env and nohup to the player itself, so the
# player is this script's own child and `wait` returns its real status.
(cd "$(dirname "$PLAYER")" && exec env -i PATH="$(dirname "$PLAYER")" HOME="$S/absent-home" TMPDIR="$S/player-tmp" \
  /usr/bin/nohup "$PLAYER" --package "$DATA" --window) >"$OUT/player.log" 2>&1 </dev/null &
NEW=$!
i=0
until ps -o command= -p "$NEW" 2>/dev/null | grep -q "^$PLAYER --package"; do
  sleep 0.1; i=$((i + 1))
  [ $i -ge 50 ] && { echo "pid $NEW is not the player: $(ps -o command= -p "$NEW")"; break; }
done
echo "$NEW" >"$S/main.pid"
echo "$RUN" >"$S/current.run"
echo "$(date -u +%FT%TZ) launched $RUN pid $NEW ($(ps -o command= -p "$NEW")) sha256 $actual; supervisor pid $$" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.25; i=$((i + 1))
  [ $i -ge 120 ] && { echo "no window after 30 s"; break; }
done
# The tao window takes the front at launch; return it and the pointer at once.
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  $W activate "$PREV" >/dev/null
  $W warp $CURSOR
fi
FRAME=$($W "$NEW" window 2>&1)
SCALE=$($W "$NEW" scale 2>&1)
# Startup content is 640 x 480 px, i.e. 480/scale pt high; the rest of the frame is the title bar.
echo "$FRAME" | python3 -I -c 'import json,sys; print(int(round(json.load(sys.stdin)["h"] - 480 / float(sys.argv[1]))))' "$SCALE" >"$S/titlebar.pt"
echo "$(date -u +%FT%TZ) $RUN pid $NEW window $FRAME scale $SCALE titlebar $(cat "$S/titlebar.pt") pt; front returned to $($W frontmost) (waited ${waited}s)" | tee -a "$S/processes.log"
wait "$NEW"
status=$?
ended=$(date -u +%FT%TZ)
rm -f "$S/main.pid"
reports=$(find "$HOME/Library/Logs/DiagnosticReports" -maxdepth 1 -name 'incant_game_player*' -newer "$OUT/launched.marker" 2>/dev/null | tr '\n' ' ')
python3 -I -c '
import json, sys
run, pid, status, ended, reports, log = sys.argv[1:7]
lines = [l for l in open(log, encoding="utf-8", errors="replace").read().splitlines() if l.startswith("{")]
report = None
for line in reversed(lines):
    try:
        report = json.loads(line); break
    except ValueError:
        pass
status = int(status)
out = {"run": run, "pid": int(pid), "exit_status": status,
       "signal": status - 128 if status > 128 else None, "ended_at": ended,
       "native_report_printed": report is not None, "report": report,
       "new_crash_reports": reports.split()}
json.dump(out, open(sys.argv[7], "w", encoding="utf-8"), indent=2, ensure_ascii=False)
print(json.dumps({k: out[k] for k in ("run", "pid", "exit_status", "signal", "native_report_printed", "new_crash_reports")}))
' "$RUN" "$NEW" "$status" "$ended" "$reports" "$OUT/player.log" "$OUT/exit.json"
echo "$ended $RUN pid $NEW exited with status $status; crash reports since launch: ${reports:-none}" | tee -a "$S/processes.log"
