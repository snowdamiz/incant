#!/bin/sh
# probe.sh CASES_JSON OUT_DIR: run layout_probe.rs through the public incant_text API.
# The probe is copied into crates/incant_text/tests only for this run and always removed;
# the script then proves that no file under crates/ changed. CPU only; ci profile; -j2.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
DEST=$ROOT/crates/incant_text/tests/zz_0055_layout_probe.rs
[ $# -eq 2 ] || { echo "usage: probe.sh CASES_JSON OUT_DIR"; exit 2; }
[ -e /tmp/incant0055/released ] && { echo "0055 lease released; no build"; exit 2; }
[ -e "$DEST" ] && { echo "probe copy already present at $DEST"; exit 2; }
CASES=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
mkdir -p "$2"
OUT=$(cd "$2" && pwd)
cp "$HERE/layout_probe.rs" "$DEST"
trap 'rm -f "$DEST"' EXIT INT TERM
status=0
INCANT0055_CASES=$CASES INCANT0055_OUT=$OUT CARGO_TARGET_DIR=/tmp/incant-agent-native-target \
  "$ROOT/tools/cargo" test --locked --offline -j2 --profile ci -p incant_text \
  --test zz_0055_layout_probe -- --nocapture || status=$?
rm -f "$DEST"
changed=$(cd "$ROOT" && git status --porcelain -- crates)
[ -z "$changed" ] || { echo "unexpected crates/ changes: $changed"; exit 3; }
echo "probe exit $status; crates/ unchanged"
exit $status
