#!/bin/sh
# Launches the prepared review build only through its sandbox profile, after
# checking the executable hash, and prints the app PID. Never uses `open`.
set -eu
ROOT=/private/tmp/incant-native-review-28e5491
EXE="$ROOT/Incant Review 28e5491.app/Contents/MacOS/incant_review_28e5491"
PROJECT="$ROOT/fixture/review.incant.json"
EXPECTED=51599f65f3e96458d706a81c81b71bc392e93812923f11549e4ae0bd3024b79e
LOG=${1:?usage: launch.sh <log file>}

actual=$(shasum -a 256 "$EXE" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "executable hash mismatch: $actual" >&2; exit 1; }

# sandbox-exec applies the profile and then execs the editor, so $! is the editor's PID.
nohup /usr/bin/sandbox-exec -f "$ROOT/review.sb" "$EXE" "$PROJECT" >"$LOG" 2>&1 &
echo $!
