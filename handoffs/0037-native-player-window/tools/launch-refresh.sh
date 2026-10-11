#!/bin/sh
# Launch the immutable corrected 0037 player (delta review) with stdout/stderr
# captured, recording the exact game PID and its exit status. Run from the
# worktree root:
#   handoffs/0037-native-player-window/tools/launch-refresh.sh OUTDIR static.scene|stream.scene [extra args]
set -u
out=$1
scene=$2
shift 2
dir=artifacts/player-refresh
bin=$dir/incant_game_player
expected=ec18e426c67584c4ac762cf72deda6dfc10d9044c4a5ff91a07fc4ad0648edf5
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
"$bin" --window "$dir/$scene" "$dir/audio" "$dir/clips.bin" "$dir/actions.json" "$dir/models" \
  "$dir/textures" 00000000000000000000000000000004 "$@" >"$out/stdout.txt" 2>"$out/stderr.txt" &
pid=$!
echo "$pid" >"$out/pid.txt"
date -u +%Y-%m-%dT%H:%M:%SZ >"$out/started.txt"
wait "$pid"
status=$?
echo "$status" >"$out/exit.txt"
date -u +%Y-%m-%dT%H:%M:%SZ >"$out/ended.txt"
echo "pid=$pid exit=$status"
