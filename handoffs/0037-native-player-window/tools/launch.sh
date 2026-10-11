#!/bin/sh
# Launch the immutable 0037 player with stdout/stderr captured, recording the
# exact game PID and its exit status. Run from the worktree root:
#   handoffs/0037-native-player-window/tools/launch.sh OUTDIR static.scene|stream.scene [extra args]
set -u
out=$1
scene=$2
shift 2
bin=artifacts/player/incant_game_player
expected=6b6a53a1ee86fabc9b2c98c7dd4601534bd72521c828bd55c779dbafbe5b3560
actual=$(shasum -a 256 "$bin" | cut -d ' ' -f 1)
if [ "$actual" != "$expected" ]; then
  echo "player hash mismatch: $actual" >&2
  exit 3
fi
if [ -e "$out/pid.txt" ]; then
  echo "refusing to reuse $out" >&2
  exit 3
fi
mkdir -p "$out"
"$bin" --window "artifacts/player/$scene" artifacts/player/audio artifacts/player/clips.bin \
  artifacts/player/actions.json artifacts/player/models artifacts/player/textures \
  00000000000000000000000000000004 "$@" >"$out/stdout.txt" 2>"$out/stderr.txt" &
pid=$!
echo "$pid" >"$out/pid.txt"
date -u +%Y-%m-%dT%H:%M:%SZ >"$out/started.txt"
wait "$pid"
status=$?
echo "$status" >"$out/exit.txt"
date -u +%Y-%m-%dT%H:%M:%SZ >"$out/ended.txt"
echo "pid=$pid exit=$status"
