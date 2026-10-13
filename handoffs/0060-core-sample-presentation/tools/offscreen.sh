#!/bin/sh
# offscreen.sh DATA PLAN OUT [GUEST]: run review_offscreen.rs through the GPU consumer offscreen.
# GUEST: omitted (logging stand-in), a CommonJS probe file, or `cooked` (the package's own cooked
# program: DATA/../../producer/behavior.rs). The harness and that behavior.rs are copied into
# crates/incant_render_gpu_host/examples only for this run and always removed; the script then
# proves no file under that crate changed. ci profile, -j2, leased target only (cargo.sh).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
. "$HERE/guard.sh"
lease_guard
DEST=$ROOT/crates/incant_render_gpu_host/examples/zz_0060_review.rs
[ $# -ge 3 ] || { echo "usage: offscreen.sh DATA PLAN OUT [GUEST_JS]"; exit 2; }
[ -e "$DEST" ] && { echo "harness copy already present at $DEST"; exit 2; }
DATADIR=$ROOT/crates/incant_render_gpu_host/examples/zz_0060_data
[ -e "$DATADIR" ] && { echo "harness data already present at $DATADIR"; exit 2; }
BEHAVIOR=$(cd "$1/../.." && pwd)/producer/behavior.rs
[ -f "$BEHAVIOR" ] || { echo "package has no producer/behavior.rs snapshot"; exit 2; }
mkdir "$DATADIR"
cp "$BEHAVIOR" "$DATADIR/behavior.rs"
cp "$HERE/review_offscreen.rs" "$DEST"
trap 'rm -f "$DEST"; rm -rf "$DATADIR"' EXIT INT TERM
status=0
"$HERE/cargo.sh" run --offline --profile ci -p incant_render_gpu_host --example zz_0060_review -- "$@" || status=$?
rm -f "$DEST"; rm -rf "$DATADIR"
changed=$(cd "$ROOT" && git status --porcelain -- crates/incant_render_gpu_host)
[ -z "$changed" ] || { echo "unexpected incant_render_gpu_host changes: $changed"; exit 3; }
echo "offscreen exit $status; incant_render_gpu_host unchanged"
exit $status
