#!/usr/bin/env bash
# Reproduce one complete 0029 evidence set into a NEW directory (repo root):
#   caffeinate -s -i bash handoffs/0029-grid-navigation-lookdev/tools/run_set.sh artifacts/0029-grid-navigation/A
set -euo pipefail
out="${1:?usage: run_set.sh NEW_OUT_DIR}"
here="$(cd "$(dirname "$0")" && pwd)"
python3 -I "$here/grid_course.py" "$out" --ticks 540 \
  --run plan:plan:5 \
  --run oblique:oblique:6 \
  --run gate:gate:6 \
  --window gauge:gauge:probe-1:0:4:1 \
  --window door-close:door:close_a:12:48:1 \
  --window gate-close:gate:close_b:12:120:2 \
  --window reopen:gate:reopen:10:72:1 \
  --window arrive:oblique:arrive:48:72:2
