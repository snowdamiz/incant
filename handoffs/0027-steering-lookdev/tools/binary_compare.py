#!/usr/bin/env python3
"""Compare the same course run on two binaries, from committed script logs only.

Usage:
  binary_compare.py PRIOR_RUN_DIR NEW_RUN_DIR [--long LONG_RUN_DIR] [--json OUT]

Each RUN_DIR holds geometry.json and logs.logs.jsonl written by steering_plaza.py
(`--run logs:none:1`). Reports, for each run:
  arrived_by_tick   walkers within 5 cm of their goal, every 200 ticks
  plinth_mill       walkers whose centre is within 3 m of the plinth centre
  goal_orbits       walkers whose bearing from their own goal winds through at
                    least one full turn during the last 1000 ticks (sign gives
                    the sense; bearing = atan2(z - goal_z, x - goal_x))
  remaining         remaining-distance buckets at the final tick
plus the first tick at which the two runs' positions differ.
"""
import argparse
import collections
import json
import math
from pathlib import Path

ARRIVE = 0.05


def load(run_dir):
    run_dir = Path(run_dir)
    walkers = sorted(json.loads((run_dir / "geometry.json").read_text())["walkers"], key=lambda w: w["name"])
    positions = {}
    for line in (run_dir / "logs.logs.jsonl").read_text().splitlines():
        entry = json.loads(line)
        if entry["level"] == "debug":
            m = json.loads(entry["message"])
            positions[m["t"]] = [(m["p"][2 * i] / 1000, m["p"][2 * i + 1] / 1000) for i in range(len(m["p"]) // 2)]
    return walkers, positions


def describe(walkers, positions):
    last = max(positions)
    goals = [tuple(w["goal"]) for w in walkers]
    arrived = [(t, sum(math.dist(positions[t][i], goals[i]) < ARRIVE for i in range(len(walkers))))
               for t in range(200, last + 1, 200)] + [(last, sum(math.dist(positions[last][i], goals[i]) < ARRIVE
                                                                  for i in range(len(walkers))))]
    mill = [(t, sum(math.hypot(*q) < 3 for q in positions[t])) for t in range(200, last + 1, 200)]
    orbits = []
    for i, w in enumerate(walkers):
        total, prev = 0.0, None
        for t in range(max(1, last - 1000), last + 1):
            x, z = positions[t][i]
            gx, gz = goals[i]
            if math.hypot(x - gx, z - gz) < ARRIVE:
                prev = None
                continue
            a = math.atan2(z - gz, x - gx)
            if prev is not None:
                total += (a - prev + math.pi) % (2 * math.pi) - math.pi
            prev = a
        if abs(total) >= 2 * math.pi:
            orbits.append((round(math.degrees(total)), w["name"]))
    buckets = collections.Counter()
    for i in range(len(walkers)):
        r = math.dist(positions[last][i], goals[i])
        buckets["<0.05" if r < 0.05 else "<0.6" if r < 0.6 else "<1.5" if r < 1.5 else "<5" if r < 5 else ">=5"] += 1
    return {"last_tick": last, "arrived_by_tick": arrived, "plinth_mill": mill,
            "goal_orbits_last_1000_ticks": {"count": len(orbits),
                                            "clockwise": sum(o < 0 for o, _ in orbits),
                                            "largest": sorted(orbits, key=lambda o: -abs(o[0]))[:6]},
            "remaining_at_last_tick": dict(sorted(buckets.items()))}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("prior")
    parser.add_argument("new")
    parser.add_argument("--long")
    parser.add_argument("--json")
    args = parser.parse_args()
    pw, pp = load(args.prior)
    nw, np_ = load(args.new)
    assert [w["name"] for w in pw] == [w["name"] for w in nw], "different walker sets"
    common = sorted(set(pp) & set(np_))
    first = next((t for t in common if pp[t] != np_[t]), None)
    result = {"first_differing_tick": first, "prior": describe(pw, pp), "new": describe(nw, np_)}
    if args.long:
        result["new_long"] = describe(*load(args.long))
    text = json.dumps(result, indent=1)
    if args.json:
        Path(args.json).write_text(text + "\n", encoding="utf-8")
    print(text)


if __name__ == "__main__":
    main()
