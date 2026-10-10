#!/usr/bin/env python3
"""Independent per-tick checks of actual walker motion for handoff 0027.

Reads only engine outputs: geometry.json from the helper and the committed
script logs (`--log-output`). The behavior logs every walker's start-of-tick
position in millimetres; this script recomputes everything from those
positions (it does not trust the behavior's own diagnostic row, but it does
cross-check against it):

  separation   minimum centre distance over all 4950 pairs (overlap < 0.50 m,
               inside the 0.05 m margin < 0.60 m)
  clearance    gap from each walker's 0.25 m footprint to the plinth and posts
  speed        actual displacement between consecutive ticks x 60 Hz
  arrival      within 5 cm of the goal; first arrival and arrived-at-end
  stalls       not arrived and actual speed < 0.1 m/s; longest run per walker

Positions are rounded to 1 mm in the log, so distances carry up to ~1.4 mm of
rounding error and per-tick speeds up to ~0.085 m/s.

Usage:
  trace_check.py summary RUN_DIR LOGS.jsonl [--csv OUT.csv] [--json OUT.json]
  trace_check.py compare DIR_A DIR_B          # frames (sha256) and logs
  trace_check.py window FULL.logs.jsonl WINDOW.logs.jsonl
"""

import argparse
import csv
import hashlib
import json
import math
import sys
from pathlib import Path

RADIUS, MARGIN, ARRIVE, STALL, HZ = 0.25, 0.05, 0.05, 0.1, 60


def load(logs):
    positions, rows, events = {}, {}, []
    for line in Path(logs).read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        message = entry["message"]
        data = json.loads(message) if message.startswith("{") else message
        if entry["level"] == "debug" and "p" in data:
            p = data["p"]
            positions[data["t"]] = [(p[2 * i] / 1000, p[2 * i + 1] / 1000) for i in range(len(p) // 2)]
        elif entry["level"] == "info" and isinstance(data, dict) and "sep" in data:
            rows[data["t"]] = data
        else:
            events.append({"tick": entry["tick"], "level": entry["level"], "message": data})
    return positions, rows, events


def polygon_gap(p, poly):
    inside, best = True, math.inf
    for i in range(len(poly)):
        (ax, az), (bx, bz) = poly[i], poly[(i + 1) % len(poly)]
        ex, ez = bx - ax, bz - az
        if ex * (p[1] - az) - ez * (p[0] - ax) < 0:
            inside = False
        t = max(0.0, min(1.0, ((p[0] - ax) * ex + (p[1] - az) * ez) / (ex * ex + ez * ez)))
        best = min(best, math.hypot(p[0] - ax - t * ex, p[1] - az - t * ez))
    return (-best if inside else best) - RADIUS


def square(cx, cz, h):
    return [(cx - h, cz - h), (cx + h, cz - h), (cx + h, cz + h), (cx - h, cz + h)]


def summary(run_dir, logs, csv_out=None, json_out=None):
    geometry = json.loads((Path(run_dir) / "geometry.json").read_text(encoding="utf-8"))
    walkers = sorted(geometry["walkers"], key=lambda w: w["name"])
    names = [w["name"] for w in walkers]
    goals = [tuple(w["goal"]) for w in walkers]
    plinth = [tuple(v) for v in geometry["plinth"]["vertices"]]
    posts = [square(x, z, geometry["post_half"]) for x, z in geometry["posts"]]
    positions, rows, events = load(logs)
    ticks = sorted(positions)
    n = len(names)
    first_arrival, stall_run, longest_stall = [None] * n, [0] * n, [(0, None)] * n
    worst = {"separation": (math.inf, None), "plinth": (math.inf, None), "post": (math.inf, None)}
    max_speed, overlap_ticks, margin_ticks, all_arrived, table = 0.0, 0, 0, None, []
    mismatches = []
    for k, t in enumerate(ticks):
        p = positions[t]
        separation, pair, overlaps, in_margin = math.inf, None, 0, 0
        for i in range(n):
            for j in range(i + 1, n):
                d = math.dist(p[i], p[j])
                if d < separation:
                    separation, pair = d, (names[i], names[j])
                overlaps += d < 2 * RADIUS
                in_margin += d < 2 * (RADIUS + MARGIN)
        plinth_gap = min(polygon_gap(q, plinth) for q in p)
        post_gap = min(polygon_gap(q, post) for q in p for post in posts)
        remaining = [math.dist(p[i], goals[i]) for i in range(n)]
        arrived = sum(d < ARRIVE for d in remaining)
        nxt = positions.get(t + 1)
        speeds = [math.dist(p[i], nxt[i]) * HZ for i in range(n)] if nxt else None
        stalled = 0
        for i in range(n):
            if remaining[i] < ARRIVE and first_arrival[i] is None:
                first_arrival[i] = t
            if speeds is not None:
                if remaining[i] >= ARRIVE and speeds[i] < STALL:
                    stall_run[i] += 1
                    stalled += 1
                    if stall_run[i] > longest_stall[i][0]:
                        longest_stall[i] = (stall_run[i], t - stall_run[i] + 1)
                else:
                    stall_run[i] = 0
        if speeds:
            max_speed = max(max_speed, max(speeds))
        overlap_ticks += overlaps > 0
        margin_ticks += in_margin > 0
        if arrived == n and all_arrived is None:
            all_arrived = t
        for key, value in (("separation", separation), ("plinth", plinth_gap), ("post", post_gap)):
            if value < worst[key][0]:
                worst[key] = (value, t)
        row = rows.get(t)
        if row:
            kinds = [k for k, bad in (("separation", abs(row["sep"] - separation) > 0.003),
                                      ("arrived", row["arrived"] != arrived),
                                      ("plinth_gap", abs(row["plinth_gap"] - plinth_gap) > 0.003)) if bad]
            if kinds:
                mismatches.append({"t": t, "kinds": kinds, "arrived_delta": row["arrived"] - arrived,
                                   "behavior": row, "independent": [separation, arrived, plinth_gap]})
        table.append({
            "tick": t, "min_separation_m": round(separation, 3), "overlap_pairs": overlaps,
            "pairs_inside_margin": in_margin, "plinth_gap_m": round(plinth_gap, 3), "post_gap_m": round(post_gap, 3),
            "speed_max_mps": round(max(speeds), 3) if speeds else "",
            "speed_mean_mps": round(sum(speeds) / n, 3) if speeds else "",
            "arrived": arrived, "stalled": stalled, "longest_current_stall": max(stall_run),
            "remaining_max_m": round(max(remaining), 3),
        })
    stalls = sorted(((s, start, names[i]) for i, (s, start) in enumerate(longest_stall) if s), reverse=True)
    last = positions[ticks[-1]]
    result = {
        "ticks": [ticks[0], ticks[-1]], "walkers": n,
        "min_separation_m": round(worst["separation"][0], 4), "min_separation_tick": worst["separation"][1],
        "overlap_ticks": overlap_ticks, "ticks_with_pairs_inside_margin": margin_ticks,
        "min_plinth_gap_m": round(worst["plinth"][0], 4), "min_plinth_gap_tick": worst["plinth"][1],
        "min_post_gap_m": round(worst["post"][0], 4), "min_post_gap_tick": worst["post"][1],
        "max_actual_speed_mps": round(max_speed, 3),
        "all_arrived_tick": all_arrived,
        "arrived_at_end": sum(math.dist(last[i], goals[i]) < ARRIVE for i in range(n)),
        "never_arrived": [names[i] for i in range(n) if first_arrival[i] is None],
        "first_arrival_range": [min(a for a in first_arrival if a), max(a for a in first_arrival if a)]
        if any(first_arrival) else None,
        "latest_first_arrivals": sorted(((a, names[i]) for i, a in enumerate(first_arrival) if a), reverse=True)[:6],
        "longest_stalls_ticks_start_walker": stalls[:6],
        "walkers_with_stall_over_60_ticks": sum(s > 60 for s, _, _ in stalls),
        "max_detour_m": max_detour(positions, walkers),
        # The behavior measures exact float positions; this script sees 1 mm rounded
        # ones, so walkers within ~1 mm of the 5 cm arrival radius can flip.
        "behavior_row_mismatches": {kind: sum(kind in m["kinds"] for m in mismatches)
                                    for kind in ("separation", "arrived", "plinth_gap")},
        "max_arrived_count_difference": max((abs(m["arrived_delta"]) for m in mismatches), default=0),
        "first_mismatches": mismatches[:3],
        "events": events[:20],
    }
    if csv_out:
        with open(csv_out, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=list(table[0]), lineterminator="\n")
            writer.writeheader()
            writer.writerows(table)
    if json_out:
        Path(json_out).write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=1))
    return result


def max_detour(positions, walkers):
    """Largest lateral distance from the straight start-goal line, per group."""
    names = [w["name"] for w in sorted(walkers, key=lambda w: w["name"])]
    ordered = sorted(walkers, key=lambda w: w["name"])
    best = {}
    for i, w in enumerate(ordered):
        (sx, sz), (gx, gz) = w["start"], w["goal"]
        ux, uz = gx - sx, gz - sz
        length = math.hypot(ux, uz)
        dev = max(abs((p[i][0] - sx) * uz - (p[i][1] - sz) * ux) / length for p in positions.values())
        group = names[i].split()[1]
        if dev > best.get(group, (0, ""))[0]:
            best[group] = (round(dev, 2), names[i])
    return best


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compare(a, b):
    a, b = Path(a), Path(b)
    report = {}
    for frames in sorted(p for p in a.iterdir() if p.is_dir() and (p / "report.json").exists()):
        other = b / frames.name
        names = sorted(f.name for f in frames.glob("*.png"))
        same = [n for n in names if (other / n).exists() and digest(frames / n) == digest(other / n)]
        report[frames.name] = {"frames": len(names), "identical": len(same),
                               "other_frames": len(list(other.glob("*.png")))}
    for logs in sorted(a.glob("*.logs.jsonl")):
        report[logs.name] = {"identical": (b / logs.name).exists() and digest(logs) == digest(b / logs.name),
                             "bytes": logs.stat().st_size}
    print(json.dumps(report, indent=1))
    return report


def window(full, part):
    """A --load-save window must log exactly what the uninterrupted run logged."""
    full_lines = [json.loads(l) for l in Path(full).read_text(encoding="utf-8").splitlines()]
    part_lines = [json.loads(l) for l in Path(part).read_text(encoding="utf-8").splitlines()]
    ticks = {e["tick"] for e in part_lines}
    expected = [e for e in full_lines if e["tick"] in ticks]
    strip = lambda es: [(e["tick"], e["level"], e["message"]) for e in es]
    result = {"window_ticks": [min(ticks), max(ticks)], "entries": len(part_lines),
              "identical_to_full_run": strip(expected) == strip(part_lines)}
    print(json.dumps(result))
    return result


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("summary")
    s.add_argument("run_dir")
    s.add_argument("logs")
    s.add_argument("--csv")
    s.add_argument("--json")
    c = sub.add_parser("compare")
    c.add_argument("a")
    c.add_argument("b")
    w = sub.add_parser("window")
    w.add_argument("full")
    w.add_argument("part")
    args = parser.parse_args()
    if args.cmd == "summary":
        summary(args.run_dir, args.logs, args.csv, args.json)
    elif args.cmd == "compare":
        compare(args.a, args.b)
    else:
        sys.exit(0 if window(args.full, args.part)["identical_to_full_run"] else 1)


if __name__ == "__main__":
    main()
