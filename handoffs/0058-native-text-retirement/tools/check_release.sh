#!/bin/sh
# check_release.sh: after the 0058 release, run every guarded tool once with harmless arguments
# and require a refusal (non-zero exit) that changes nothing in the evidence directory (file
# list, sizes and modification times compared before and after). Then repeat with fake homes
# under the released directory's tmp/guard-tests holding an expired, a foreign and no lease,
# to exercise the expiry and packet checks. Prints one line per tool and a final verdict.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REAL=$HOME/.codex/incant-evidence/0058
[ -e "$REAL/released" ] || { echo "not released yet; nothing to check"; exit 2; }
listing() { find "$1" -print0 | xargs -0 stat -f '%N %z %m' | sort; }
failures=0
probe() { # probe <label> <command...>: the command must fail
  label=$1; shift
  out=$("$@" 2>&1 </dev/null); status=$?
  first=$(printf '%s\n' "$out" | head -1)
  if [ $status -eq 0 ]; then failures=$((failures + 1)); verdict=ACCEPTED; else verdict=refused; fi
  echo "$label: $verdict (exit $status) $first"
}
suite() { # suite <home>
  H=$1
  probe setup.sh env HOME="$H" "$HERE/setup.sh"
  probe "build.py headless" env HOME="$H" python3 -B "$HERE/build.py" headless "$H/.codex/incant-evidence/0058/guard-build"
  probe "build.py player" env HOME="$H" python3 -B "$HERE/build.py" player "$REAL/fixture-1/authoring/project.json" \
    "$H/.codex/incant-evidence/0058/guard-pkg" --camera 00000000000000000000005810
  probe fixture.py env HOME="$H" python3 -I -B "$HERE/fixture.py" "$H/.codex/incant-evidence/0058/guard-fixture" \
    --binary "$REAL/headless-1/incant_headless"
  probe launch.sh env HOME="$H" "$HERE/launch.sh" guard-run
  probe burst.sh env HOME="$H" "$HERE/burst.sh" 'echo burst ran'
  probe cap.sh env HOME="$H" "$HERE/cap.sh" guard-capture
  probe heap.sh env HOME="$H" "$HERE/heap.sh" guard-census
  for command in idle locked frontmost cursor displays sources "warp 0 0" "hold 0" "activate 1" "1 window" "1 raise" "1 close" "1 key 48"; do
    # shellcheck disable=SC2086
    probe "winctl $command" env HOME="$H" "$REAL/winctl" $command
  done
}
echo "== real home, lease released"
before=$(listing "$REAL")
suite "$HOME"
after=$(listing "$REAL")
if [ "$before" = "$after" ]; then echo "evidence directory unchanged"; else echo "evidence directory CHANGED"; failures=$((failures + 1)); fi
FAKE=$REAL/tmp/guard-tests
for case in expired foreign missing; do
  H=$FAKE/$case
  mkdir -p "$H/.codex/incant-evidence/0058"
  if [ $case != missing ]; then
    python3 -I - "$H/.codex/incant-evidence/0058/lease.json" "$case" <<'EOF'
import json, sys
path, case = sys.argv[1:3]
lease = {"packet": "0058-native-text-retirement" if case == "expired" else "0057-native-window-game-ui",
         "expires_at": "2026-10-12T16:00:00+00:00" if case == "expired" else "2099-01-01T00:00:00+00:00",
         "cargo_target_dir": "unused"}
json.dump(lease, open(path, "x", encoding="utf-8"))
EOF
  fi
  echo "== fake home, $case lease"
  before=$(listing "$H")
  suite "$H"
  after=$(listing "$H")
  if [ "$before" = "$after" ]; then echo "fake home unchanged"; else echo "fake home CHANGED"; failures=$((failures + 1)); fi
done
echo "verdict: $failures failure(s)"
[ $failures -eq 0 ]
