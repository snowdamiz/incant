#!/bin/sh
# cargo.sh SUBCOMMAND [ARGS...]: the only cargo entry point for 0060. Runs ./tools/cargo from the
# worktree with the leased CARGO_TARGET_DIR, locked Rust 1.99.0 (rust-toolchain.toml), --locked
# and -j2 (ci profile is passed explicitly by callers), and appends start/exit records to the
# packet ledger. Refuses without a valid 0060 lease or after release.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
. "$HERE/guard.sh"
lease_guard
[ $# -ge 1 ] || { echo "usage: cargo.sh SUBCOMMAND [ARGS...]" >&2; exit 2; }
sub=$1; shift
case "$sub" in build|test|clippy|run|check) ;; *) echo "cargo.sh: unsupported subcommand $sub" >&2; exit 2;; esac
PATH=/opt/homebrew/opt/rustup/bin:$PATH
export PATH CARGO_TARGET_DIR="$TARGET"
start=$(date -u +%FT%TZ)
/usr/bin/python3 -I -c 'import json,sys; print(json.dumps({"event":"start","at":sys.argv[1],"pid":int(sys.argv[2]),"argv":sys.argv[3:]}))' \
  "$start" "$$" "$sub" --locked -j2 "$@" >>"$S/commands.jsonl"
status=0
(cd "$ROOT" && exec ./tools/cargo "$sub" --locked -j2 "$@") || status=$?
/usr/bin/python3 -I -c 'import json,sys; print(json.dumps({"event":"exit","at":sys.argv[1],"pid":int(sys.argv[2]),"status":int(sys.argv[3])}))' \
  "$(date -u +%FT%TZ)" "$$" "$status" >>"$S/commands.jsonl"
exit $status
