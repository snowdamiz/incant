#!/bin/sh
# offscreen.sh DATA PLAN OUT: run review_offscreen.rs through the GPU consumer offscreen. The file
# is copied into crates/incant_render_gpu_host/examples only for this run and always removed; the
# script then proves no file under that crate changed. ci profile, -j2, the leased target only.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
S=$HOME/.codex/incant-evidence/0057
DEST=$ROOT/crates/incant_render_gpu_host/examples/zz_0057_review.rs
[ $# -eq 3 ] || { echo "usage: offscreen.sh DATA PLAN OUT"; exit 2; }
[ -e "$S/released" ] && { echo "0057 lease released; no GPU run"; exit 2; }
python3 -I -c 'import json,sys; g=json.load(open(sys.argv[1])); sys.exit(0 if g.get("packet")=="0057-native-window-game-ui" else 1)' "$S/lease.json" \
  || { echo "no 0057 lease marker"; exit 2; }
[ -e "$DEST" ] && { echo "harness copy already present at $DEST"; exit 2; }
cp "$HERE/review_offscreen.rs" "$DEST"
trap 'rm -f "$DEST"' EXIT INT TERM
status=0
CARGO_TARGET_DIR=$HOME/.codex/incant-native-target "$ROOT/tools/cargo" run --locked --offline -j2 --profile ci \
  -p incant_render_gpu_host --example zz_0057_review -- "$1" "$2" "$3" || status=$?
rm -f "$DEST"
changed=$(cd "$ROOT" && git status --porcelain -- crates/incant_render_gpu_host)
[ -z "$changed" ] || { echo "unexpected incant_render_gpu_host changes: $changed"; exit 3; }
echo "offscreen exit $status; incant_render_gpu_host unchanged"
exit $status
