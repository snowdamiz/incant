#!/bin/bash
# Guarded cargo runner for handoff 0059: exclusive warm target, pinned
# toolchain, full log and a ledger line (argv, exit status, log hash) per run.
# Usage: tools/cargo.sh <label> <cargo args...>
set -u
E=/Users/andreyyurlov/.codex/incant-evidence/0059
TARGET=/Users/andreyyurlov/.codex/incant-native-target
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
label=$1
shift
if [ -e "$E/released" ] || [ ! -f "$E/lease.json" ]; then
    echo "0059 lease released or missing; refusing" >&2
    exit 97
fi
log="$E/logs/$label.log"
if [ -e "$log" ]; then
    echo "log $log exists; choose a new label" >&2
    exit 98
fi
start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cd "$ROOT" || exit 99
# rustup proxies; rust-toolchain.toml selects 1.99.0.
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
{
    echo "# label: $label"
    echo "# cwd: $ROOT"
    echo "# CARGO_TARGET_DIR=$TARGET cargo $*"
    rustc -vV | sed 's/^/# /'
    echo "# start: $start"
} >"$log"
CARGO_TARGET_DIR=$TARGET cargo "$@" >>"$log" 2>&1 &
child=$!
echo "pid $child -> $log"
wait "$child"
status=$?
end=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "# exit: $status at $end" >>"$log"
hash=$(shasum -a 256 "$log" | cut -d' ' -f1)
python3 -I -c 'import json,sys; print(json.dumps({"label":sys.argv[1],"pid":int(sys.argv[2]),"argv":sys.argv[3],"start":sys.argv[4],"end":sys.argv[5],"exit":int(sys.argv[6]),"log":sys.argv[7],"log_sha256":sys.argv[8]}))' \
    "$label" "$child" "$*" "$start" "$end" "$status" "$log" "$hash" >>"$E/commands.jsonl"
echo "exit $status; log sha256 $hash"
exit "$status"
