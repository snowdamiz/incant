#!/bin/sh
# nav_repro.sh X Y Z X Y Z ... | --sweep: generate and run the navmesh reproduction example once per
# NAV_RUNS entry ("quiet CELL CELL_H TILE CLIMB;..."; default: the level's own settings). The example is
# copied into crates/incant_physics_runtime/examples only for this run and always removed; the
# script then proves no file under that crate changed. ci profile, -j2, leased target (cargo.sh).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
. "$HERE/guard.sh"
lease_guard
DEST=$ROOT/crates/incant_physics_runtime/examples/zz_0060_nav.rs
[ -e "$DEST" ] && { echo "example already present at $DEST"; exit 2; }
mkdir -p "$ROOT/crates/incant_physics_runtime/examples"
/usr/bin/python3 -B "$HERE/nav_repro.py" "$DEST" "$@"
trap 'rm -f "$DEST"; rmdir "$ROOT/crates/incant_physics_runtime/examples" 2>/dev/null || true' EXIT INT TERM
status=0
"$HERE/cargo.sh" build --offline --profile ci -p incant_physics_runtime --example zz_0060_nav || status=$?
if [ $status -eq 0 ]; then
  BIN=$TARGET/ci/examples/zz_0060_nav
  if [ -z "${NAV_RUNS:-}" ]; then "$BIN" || status=$?; else
    OLDIFS=$IFS; IFS=';'; for run in $NAV_RUNS; do IFS=$OLDIFS; "$BIN" $run | tail -8 || status=$?; IFS=';'; done; IFS=$OLDIFS
  fi
fi
rm -f "$DEST"; rmdir "$ROOT/crates/incant_physics_runtime/examples" 2>/dev/null || true
changed=$(cd "$ROOT" && git status --porcelain -- crates/incant_physics_runtime)
[ -z "$changed" ] || { echo "unexpected incant_physics_runtime changes: $changed"; exit 3; }
echo "nav repro exit $status; incant_physics_runtime unchanged"
exit $status
