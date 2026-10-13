#!/bin/sh
# variants.sh ART_DIR TAG "locale:state ...": build each presentation variant with
# samples/core/scene.py (public commands only) and package it with package.py (ci window
# player in the leased target). Outputs: ~/.codex/incant-evidence/0060/builds/TAG-locale-state
# and packages/TAG-locale-state. Stops at the first failure.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
. "$HERE/guard.sh"
lease_guard
ART=$1; TAG=$2; shift 2
for variant in $*; do
  locale=${variant%%:*}; state=${variant##*:}
  name=$TAG-$locale-$state
  echo "== $name"
  (cd "$ROOT" && python3 -B samples/core/scene.py "$S/builds/$name" --binary "$S/bin/incant_headless" \
      --art "$ART" --locale "$locale" --state "$state" > "$S/logs/build-$name.txt" 2>&1)
  (cd "$ROOT" && python3 -B "$HERE/package.py" "$S/builds/$name/project/core.json" "$S/packages/$name" \
      --camera 01CSEN00000000000000000003 > "$S/logs/package-$name.txt" 2>&1)
  tail -1 "$S/logs/package-$name.txt"
done
