#!/usr/bin/env bash
# Reproduce one complete 0028 evidence set into a NEW directory (repo root):
#   caffeinate -s -i bash handoffs/0028-off-mesh-lookdev/tools/run_set.sh artifacts/0028-off-mesh/A
set -euo pipefail
out="${1:?usage: run_set.sh NEW_OUT_DIR}"
here="$(cd "$(dirname "$0")" && pwd)"
python3 -I "$here/off_mesh_course.py" "$out" --ticks 1000 \
  --run overview:overview:8 \
  --run profile:profile:8 \
  --window up-launch:west-gap:launch-up-1:8:80:1 \
  --window bridge-reverse:east-gap:launch-bridge-2:8:80:1 \
  --window ret-full:west-gap:launch-ret-1:8:88:1 \
  --window ret-mid:west-gap:mid-ret:0:48:1 \
  --window goal-feet:goal-feet:arrive-1:40:80:2
