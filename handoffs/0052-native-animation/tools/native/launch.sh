#!/bin/sh
# launch.sh <log> <project.json>: start the granted 0052 editor binary on the fixture while nobody
# is using the Mac, record its PID, and hand the front back. Adapted from 0050: the binary and its
# hash are 0052's, the packet's build.json must still record the native/GPU grant, a recorded
# release refuses any later launch, and no network sampler is attached (this packet asks for none).
# Node comes first in PATH because native Play cooks the fixture's TypeScript behaviour with a
# host Node (crates/incant_cook/src/typescript.rs); it is the runtime 0050 used.
set -u
S=/tmp/incant0052
W=$S/winctl
BIN=/tmp/incant-native-animation-0052-bin-v1/incant_editor
EXPECTED=4c1bf1f00e14fd5a2a539b08f2cead1787c2da2d4dbf6cbefd47090e6e0b39a1
NODE_DIR=$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin
HERE=$(cd "$(dirname "$0")" && pwd)
GRANT=$HERE/../../build.json
[ $# -eq 2 ] || { echo "usage: launch.sh <log> <project.json>"; exit 2; }
[ -e "$S/released" ] && { echo "native/GPU ownership was released; no launch"; exit 2; }
python3 -I -c 'import json,sys; sys.exit(0 if json.load(open(sys.argv[1])).get("native_gpu_granted") is True else 1)' "$GRANT" \
  || { echo "build.json does not record the native/GPU grant"; exit 2; }
actual=$(shasum -a 256 "$BIN" | cut -d' ' -f1)
[ "$actual" = "$EXPECTED" ] || { echo "binary hash $actual does not match the grant"; exit 2; }
# Anchored: the editor's argv[0] is the binary path. Unanchored, the guard matched its own
# shell command in 0048.
if pgrep -f "^$BIN" >/dev/null; then echo "an editor from this binary is already running"; exit 2; fi
waited=0
while [ "$($W idle)" -lt 15 ]; do
  sleep 2; waited=$((waited + 2))
  [ $waited -ge 900 ] && { echo "nobody idle for 15 s within 15 min; launch skipped"; exit 3; }
done
PREV=$($W frontmost)
CURSOR=$($W cursor)
PATH="$NODE_DIR:$PATH" nohup "$BIN" "$2" >"$1" 2>&1 &
NEW=$!
echo "$NEW" >"$S/main.pid"
echo "$(date -u +%FT%TZ) launched $NEW sha256 $actual node $("$NODE_DIR/node" --version)" >>"$S/processes.log"
i=0
until $W "$NEW" window >/dev/null 2>&1; do
  sleep 0.5; i=$((i + 1))
  [ $i -ge 60 ] && { echo "no window after 30 s"; exit 4; }
done
sleep 3
echo "pid $NEW window $($W "$NEW" window) (waited ${waited}s) at $(date -u +%FT%TZ)"
if [ "$($W frontmost)" = "$NEW" ] && [ "$PREV" != "$NEW" ]; then
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PREV) to true" >/dev/null 2>&1
  $W warp $CURSOR
fi
