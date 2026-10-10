#!/usr/bin/env python3
"""Every-tick review of a navigation_course.ts run against the authored geometry.

Usage: python3 -I trace_check.py RUN_DIR LOGS.jsonl [--rows FIRST LAST]

Reads RUN_DIR/geometry.json (written by navigation_lookdev.py) and the run's
JSONL log. Reports, for every tick until arrival:
  * stalls: horizontal motion below 90% of the requested step, or a requested
    step below 90% of SPEED*dt (only the final arrival tick may be short);
  * deviation of the capsule axis from the active returned polyline (XZ);
  * surface gap between the capsule and every static obstacle (XZ), with the
    barrier in its parked pose through the edit tick and closed afterwards;
  * groundedness and vertical motion (capsule centre vs. the true surface,
    excluding ticks whose axis is within radius+offset of a ridge edge, which
    are listed separately as edge transitions);
  * collisions reported by computeCharacterMotion, by entity name.
For each returned route it reports the minimum clearance of the polyline to
obstacles (sampled every 1 cm) and the bias of returned point heights against
the true analytic surface. Logged values are rounded to 1 mm.
"""

import json
import math
import sys
from pathlib import Path

SPEED, DT = 2.0, 1 / 60
EDIT_TICK = 100
OFFSET = 0.02


def footprints(geometry, closed):
    """(name, kind, data, top) for every obstacle taller than the climb limit."""
    out = []
    for piece in geometry["pieces"]:
        if piece["key"] in ("floor", "ridge"):
            continue
        center, yaw = piece["center"], piece["yaw"]
        if piece["name"] == "Barrier" and closed:
            center, yaw = geometry["barrier_closed"]["center"], geometry["barrier_closed"]["yaw"]
        if piece["shape"] == "capsule":
            out.append((piece["name"], "circle", (center[0], center[2], piece["radius"])))
        else:
            out.append((piece["name"], "box", (center[0], center[2], piece["half"][0], piece["half"][2],
                                               math.radians(yaw))))
    return out


def distance(fp, x, z):
    kind, data = fp[1], fp[2]
    if kind == "circle":
        cx, cz, r = data
        return math.hypot(x - cx, z - cz) - r
    cx, cz, hx, hz, yaw = data
    # Rotation about +Y by yaw maps local (lx, lz) to world (lx cos + lz sin, -lx sin + lz cos).
    dx, dz = x - cx, z - cz
    lx, lz = dx * math.cos(yaw) - dz * math.sin(yaw), dx * math.sin(yaw) + dz * math.cos(yaw)
    ox, oz = abs(lx) - hx, abs(lz) - hz
    outside = math.hypot(max(ox, 0), max(oz, 0))
    return outside if outside > 0 else max(ox, oz)


def surface(geometry, x, z):
    ridge = next(p for p in geometry["pieces"] if p["key"] == "ridge")
    (cx, _, cz), (hx, hy, hz) = ridge["center"], ridge["half"]
    return 2 * hy if abs(x - cx) <= hx and abs(z - cz) <= hz else 0.0


def near_ridge_edge(geometry, x, reach):
    ridge = next(p for p in geometry["pieces"] if p["key"] == "ridge")
    cx, hx = ridge["center"][0], ridge["half"][0]
    return min(abs(x - (cx - hx)), abs(x - (cx + hx))) < reach


def to_polyline(points, x, z):
    best = math.inf
    for a, b in zip(points, points[1:]):
        ax, az, bx, bz = a[0], a[2], b[0], b[2]
        length2 = (bx - ax) ** 2 + (bz - az) ** 2
        t = 0 if length2 == 0 else max(0, min(1, ((x - ax) * (bx - ax) + (z - az) * (bz - az)) / length2))
        best = min(best, math.hypot(x - (ax + t * (bx - ax)), z - (az + t * (bz - az))))
    return best


def spans(ticks):
    out = []
    for tick in ticks:
        if out and out[-1][1] == tick - 1:
            out[-1][1] = tick
        else:
            out.append([tick, tick])
    return out


def load(path):
    rows, events = [], []
    for line in open(path, encoding="utf-8"):
        record = json.loads(line)
        message = record["message"]
        if record["level"] == "debug":
            rows.append(json.loads(message))
        else:
            events.append((record["level"], json.loads(message) if message.startswith("{") else message))
    return rows, events


def route_report(geometry, route, closed):
    points = route["points"]
    clearance = {}
    for a, b in zip(points, points[1:]):
        steps = max(1, int(math.hypot(b[0] - a[0], b[2] - a[2]) / 0.01))
        for k in range(steps + 1):
            x, z = a[0] + (b[0] - a[0]) * k / steps, a[2] + (b[2] - a[2]) * k / steps
            for fp in footprints(geometry, closed):
                d = distance(fp, x, z)
                if d < clearance.get(fp[0], (math.inf,))[0]:
                    clearance[fp[0]] = (round(d, 3), [round(x, 2), round(z, 2)])
    bias = [round(p[1] - surface(geometry, p[0], p[2]), 3) for p in points]
    length = sum(math.hypot(b[0] - a[0], b[2] - a[2]) for a, b in zip(points, points[1:]))
    nearest = sorted(clearance.items(), key=lambda kv: kv[1][0])[:4]
    return {"points": len(points), "length_xz": round(length, 3), "generation": route["generation"],
            "visited": route["visited"], "corridor": route["corridor"],
            "nearest_clearance[name,(gap,at)]": nearest,
            "height_bias_vs_true_surface": {"min": min(bias), "max": max(bias), "each": bias}}


def main():
    run_dir, log = Path(sys.argv[1]), Path(sys.argv[2])
    geometry = json.loads((run_dir / "geometry.json").read_text(encoding="utf-8"))
    names = {v: k for k, v in geometry["ids"].items()}
    rows, events = load(log)
    if sys.argv[3:4] == ["--rows"]:
        first, last = int(sys.argv[4]), int(sys.argv[5])
        for row in rows:
            if first <= row["t"] <= last:
                print(json.dumps(row))
        return
    routes = {}
    arrived = None
    for level, event in events:
        if isinstance(event, dict) and event.get("event") == "route":
            routes[len(routes) + 1] = (event["t"], event["path"])
        elif isinstance(event, dict) and event.get("event") == "arrived":
            arrived = event["t"]
    print("events:", json.dumps([[lvl, e if isinstance(e, str) else {k: v for k, v in e.items() if k != "path"}
                                  | ({"null": e["path"] is None} if "path" in e else {})] for lvl, e in events]))
    for number, (tick, path) in routes.items():
        print(f"route {number} (tick {tick}):", json.dumps(route_report(geometry, path, tick > EDIT_TICK)))

    radius, rest = geometry["capsule"]["radius"], geometry["capsule"]["half_height"] + geometry["capsule"]["radius"] + OFFSET
    stalls, air, hits, worst_dev, worst_gap, gaps = [], [], {}, (0, None), (math.inf, None, None), {}
    max_dy, height_err, edge_ticks = (0, None), (0, None), []
    for row in rows:
        t, (x, y, z) = row["t"], row["p"]
        if arrived and t >= arrived:
            break
        want = math.hypot(*row["want"])
        moved = math.hypot(row["move"][0], row["move"][2])
        if want < 0.9 * SPEED * DT - 0.0005 or moved < 0.9 * want - 0.0005:
            stalls.append((t, round(want, 4), round(moved, 4)))
        if not row["g"]:
            air.append(t)
        for h in row["hit"]:
            hits.setdefault(names.get(h, h), []).append(t)
        route = routes[row["r"]][1]["points"]
        dev = to_polyline(route, x, z)
        if dev > worst_dev[0]:
            worst_dev = (round(dev, 4), t)
        for fp in footprints(geometry, t > EDIT_TICK):
            gap = distance(fp, x, z) - radius
            if gap < gaps.get(fp[0], (math.inf,))[0]:
                gaps[fp[0]] = (round(gap, 3), t)
            if gap < worst_gap[0]:
                worst_gap = (round(gap, 3), fp[0], t)
        if abs(row["move"][1]) > max_dy[0]:
            max_dy = (abs(row["move"][1]), t)
        if near_ridge_edge(geometry, x, radius + OFFSET):
            edge_ticks.append(t)
            continue
        err = y - rest - surface(geometry, x, z)
        if abs(err) > abs(height_err[0]):
            height_err = (round(err, 3), t, [x, y, z])
    final = rows[-1]["p"]
    goal = geometry["goal"]
    print("motion:", json.dumps({
        "ticks_logged": len(rows), "arrived_tick": arrived,
        "final_xz_error_to_goal": round(math.hypot(final[0] - goal[0], final[2] - goal[2]), 4),
        "stalls[t,want,moved]": stalls, "ungrounded_ticks": air,
        "collisions_by_entity": {k: [len(v), v[0], v[-1]] for k, v in hits.items()},
        "max_deviation_from_active_polyline[m,t]": worst_dev,
        "min_capsule_surface_gap[m,obstacle,t]": worst_gap,
        "per_obstacle_gap[m,t]": dict(sorted(gaps.items(), key=lambda kv: kv[1][0])),
        "max_abs_vertical_move_per_tick[m,t]": max_dy,
        "max_centre_height_error_vs_rest_over_flat_surface[m,t,p]": height_err,
        "ridge_edge_transition_tick_spans": spans(edge_ticks),
    }))


if __name__ == "__main__":
    main()
