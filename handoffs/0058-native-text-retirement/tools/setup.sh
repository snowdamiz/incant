#!/bin/sh
# setup.sh: compile winctl.swift into the 0058 evidence directory (never /tmp) and record its
# source and binary hashes in the process ledger. Guarded by the 0058 lease.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/guard.sh"
lease_guard
mkdir -p "$S/tmp"
TMPDIR="$S/tmp" /usr/bin/swiftc -O "$HERE/winctl.swift" -o "$S/winctl"
echo "$(date -u +%FT%TZ) built winctl from $(shasum -a 256 "$HERE/winctl.swift" | cut -d' ' -f1) -> $(shasum -a 256 "$S/winctl" | cut -d' ' -f1)" >>"$S/processes.log"
"$S/winctl" locked >/dev/null
echo "winctl ready: $S/winctl"
