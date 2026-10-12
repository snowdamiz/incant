#!/bin/sh
# checks.sh OUT_DIR: the 0056 final local checks on the committed HEAD, one log
# per command plus exit codes in OUT_DIR/summary.txt. GPU suites run with
# --include-ignored under the 0056 lease (Metal on this Mac); ci profile.
set -u
OUT=$1
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
[ -e /tmp/incant0056/released ] && { echo "0056 lease released; no checks"; exit 2; }
mkdir -p "$OUT"
cd "$ROOT" || exit 2
export CARGO_TARGET_DIR=/tmp/incant-agent-native-target
C="./tools/cargo"
F="--locked --offline -j2"
run() {
  name=$1; shift
  "$@" >"$OUT/$name.txt" 2>&1
  code=$?
  echo "$name exit $code: $*" | tee -a "$OUT/summary.txt"
}
echo "head $(git rev-parse HEAD) tree $(git rev-parse 'HEAD^{tree}')" >"$OUT/summary.txt"
run fmt $C fmt --all -- --check
run clippy-render $C clippy $F -p incant_ui -p incant_render_data -p incant_render -p incant_render_game \
  -p incant_render_gpu_host -p incant_render_native -p incant_play --all-targets -- -D warnings
run clippy-player $C clippy $F -p incant_game_player --all-targets -- -D warnings
run clippy-player-window $C clippy $F -p incant_game_player --all-targets --features native-window -- -D warnings
run test-ui $C test $F --profile ci -p incant_ui
run test-render-gpu $C test $F --profile ci -p incant_render -- --include-ignored --test-threads=1
run test-render-game $C test $F --profile ci -p incant_render_game -p incant_render_native -p incant_render_data
run test-gpu-host $C test $F --profile ci -p incant_render_gpu_host -- --include-ignored --test-threads=1
run test-play $C test $F --profile ci -p incant_play
run test-player-window $C test $F --profile ci -p incant_game_player --features native-window
run check-downstream $C check $F -p incant_headless -p incant_editor --all-targets
run check-wasm $C check $F -p incant_render --lib --target wasm32-unknown-unknown
echo "done" >>"$OUT/summary.txt"
