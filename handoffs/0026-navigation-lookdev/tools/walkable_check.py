#!/usr/bin/env python3
"""Compare a walkable_probe.ts map with the analytic room geometry (handoff 0026).

Usage: python3 -I walkable_check.py RUN_DIR PROBE_LOGS.jsonl [MARGIN]

A grid point is EXPECTED walkable when it lies at least MARGIN (default 0.75 m:
the 0.4 m radius plus the measured ~0.2 m erosion safety margin plus one 0.1 m
cell and slack) from every wall/pillar/barrier footprint (barrier parked; no
edit) and from the floor edge, and is not within MARGIN of a ridge face
(ridge top and floor both stay walkable, so faces are excluded only as slack).
Prints the map ('#' walkable, '.' not, 'X' expected-walkable but missing, '!'
query threw), the missing points and every logged query error.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import trace_check as tc  # noqa: E402


def main():
    run_dir, logs = Path(sys.argv[1]), Path(sys.argv[2])
    margin = float(sys.argv[3]) if len(sys.argv) > 3 else 0.75
    geometry = json.loads((run_dir / "geometry.json").read_text(encoding="utf-8"))
    records = [json.loads(line) for line in open(logs, encoding="utf-8")]
    errors = [json.loads(r["message"]) for r in records if r["level"] == "error"]
    rows = [json.loads(r["message"]) for r in records if r["level"] != "error"]
    ridge = next(p for p in geometry["pieces"] if p["key"] == "ridge")
    rx, rh = ridge["center"][0], ridge["half"][0]
    fx, _, fz = geometry["pieces"][0]["half"]
    missing = []
    print(f"x {rows[0]['xs']} step 0.2; top row = largest z; X = expected walkable but missing")
    for r in reversed(rows):
        z, x0, line = r["z"], r["xs"][0], ""
        for i, mark in enumerate(r["row"]):
            x = round(x0 + 0.2 * i, 2)
            clear = min(tc.distance(fp, x, z) for fp in tc.footprints(geometry, False))
            edge = min(fx - abs(x), fz - abs(z))
            face = min(abs(x - (rx - rh)), abs(x - (rx + rh)))
            expected = clear >= margin and edge >= margin and face >= margin
            if expected and mark == ".":
                missing.append((x, z))
                line += "X"
            else:
                line += mark
        print(f"z={z:5.1f} {line}")
    print(json.dumps({"margin": margin, "missing_count": len(missing), "missing": missing,
                      "query_errors": errors}))


if __name__ == "__main__":
    main()
