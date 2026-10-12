#!/bin/sh
# heap.sh <label>: read-only census of live Cocoa text-input objects in the running 0058 player,
# using Apple's /usr/bin/heap (it briefly suspends the process to scan its malloc zones; no
# input, focus change or window event). Saves the full listing to runs/<run>/heap-<label>.txt
# and prints the counts of tao's managed client class (`...text_input::ManagedClient<version>`)
# and NSTextInputContext. Refuses after release, without a valid lease, or when main.pid is not
# a running 0058 player.
# RETIRED after its first use: the scan suspended the background player for about 1.7 s, CoreAudio
# logged an IO overload at resume and the session ended with exit 1 (run p1-latin). It is kept
# only as the record of that probe and is not used on a live player again.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
PLAYER=$S/pkg-1/player/incant_game_player
P=$(cat "$S/main.pid")
ps -o command= -p "$P" | grep -q "^$PLAYER --package" || { echo "pid $P is not a running 0058 player"; exit 2; }
OUT=$S/runs/$(cat "$S/current.run")/heap-$1.txt
[ -e "$OUT" ] && { echo "$OUT exists"; exit 2; }
echo "$(date -u +%FT%TZ) heap census $1 of pid $P" >>"$S/processes.log"
/usr/bin/heap "$P" >"$OUT" 2>&1
status=$?
python3 -I -c '
import json, re, sys
text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
def count(pattern):
    rows = [l for l in text.splitlines() if re.search(pattern, l)]
    total = sum(int(l.split()[0]) for l in rows if l.split() and l.split()[0].isdigit())
    return total, rows
client, client_rows = count(r"text_input::ManagedClient")
context, context_rows = count(r"\bNSTextInputContext\b")
print(json.dumps({"label": sys.argv[2], "heap_exit": int(sys.argv[3]), "managed_client": client,
                  "ns_text_input_context": context, "rows": client_rows + context_rows}, ensure_ascii=False))
' "$OUT" "$1" "$status"
