#!/usr/bin/env python3
"""Independent checks of a 0029 evidence set produced by grid_course.py.

  python3 -I trace_check.py SET_DIR [--compare OTHER_SET_DIR] [--json OUT.json]

Nothing here imports engine code. The grid state at every tick is rebuilt from
the authored costs (geometry.json) plus the logged door commands, which publish
on the tick after they are issued. An exhaustive Dijkstra oracle with the
declared metric (entering weight * 1000 cardinal / * 1414 diagonal, diagonals
only when both orthogonal neighbours are walkable, start cell not charged)
checks every per-tick route cost and every probe.
"""

import argparse
import hashlib
import heapq
import json
import math
from pathlib import Path

SPEED, TURN_RATE = 3.0, 3 * math.pi  # must match grid_course.ts
EPS = 1e-9


def rows(path):
    out = []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        out.append((entry["tick"], entry["level"], json.loads(entry["message"]), entry))
    return out


def dijkstra(costs, w, h, start, end, diagonal=True):
    def cost(c):
        return costs[c[1] * w + c[0]]
    if cost(start) == 0 or cost(end) == 0:
        return None
    dist, pq = {tuple(start): 0}, [(0, tuple(start))]
    while pq:
        d, u = heapq.heappop(pq)
        if d > dist[u]:
            continue
        if u == tuple(end):
            return d
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if (dx, dy) == (0, 0) or (dx and dy and not diagonal):
                    continue
                v = (u[0] + dx, u[1] + dy)
                if not (0 <= v[0] < w and 0 <= v[1] < h) or cost(v) == 0:
                    continue
                if dx and dy and (cost((v[0], u[1])) == 0 or cost((u[0], v[1])) == 0):
                    continue
                nd = d + cost(v) * (1414 if dx and dy else 1000)
                if nd < dist.get(v, 1 << 62):
                    dist[v] = nd
                    heapq.heappush(pq, (nd, v))
    return None


def path_cost(costs, w, cells, diagonal=True):
    total = 0
    for a, b in zip(cells, cells[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        assert max(abs(dx), abs(dy)) == 1, ("non-adjacent", a, b)
        assert costs[b[1] * w + b[0]] > 0, ("blocked", b)
        if dx and dy:
            assert diagonal and costs[a[1] * w + b[0]] > 0 and costs[b[1] * w + a[0]] > 0, ("corner cut", a, b)
        total += costs[b[1] * w + b[0]] * (1414 if dx and dy else 1000)
    return total


def world(c):
    return (c[0] - 7.5, c[1] - 4.5)


def clearance(costs, w, h, x, z):
    """Distance from a world point to the nearest blocked cell square."""
    best = math.inf
    for r in range(h):
        for c in range(w):
            if costs[r * w + c]:
                continue
            x0, z0 = c - 8.0, r - 5.0
            dx = max(x0 - x, 0.0, x - (x0 + 1))
            dz = max(z0 - z, 0.0, z - (z0 + 1))
            best = min(best, math.hypot(dx, dz))
    return best


def stdout_core(path):
    d = json.loads(Path(path).read_text(encoding="utf-8"))
    return {"state": d["state"], "script_state": d["script_state"]}


def normalized_core(out):
    """Final state with init/import-generated IDs (project, scene, assets) replaced by
    stable aliases, so independently authored projects can be compared exactly.

    Refuses to normalize unless the project has exactly one scene and every asset
    name is unique, so no two generated IDs can collapse onto one alias. Only these
    generated IDs are mapped; everything else must already be equal."""
    project = json.loads((Path(out) / "grid.incant.json").read_text(encoding="utf-8"))
    if len(project["scenes"]) != 1:
        raise SystemExit(f"{out}: expected exactly one scene, found {len(project['scenes'])}")
    asset_names = [v["name"] for v in project["assets"].values()]
    if len(set(asset_names)) != len(asset_names):
        raise SystemExit(f"{out}: asset names are not unique; refusing to alias")
    names = {project["id"]: "PROJECT"}
    names.update({s: "SCENE" for s in project["scenes"]})
    names.update({a: f"ASSET:{v['name']}" for a, v in project["assets"].items()})
    if len(names) != 2 + len(asset_names):
        raise SystemExit(f"{out}: generated IDs are not distinct")
    text = json.dumps(stdout_core(Path(out) / "reference.stdout.json"), sort_keys=True)
    for ident, name in names.items():
        text = text.replace(ident, name)
    return json.loads(text), sorted(names.values())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check_set(out):
    out = Path(out)
    geo = json.loads((out / "geometry.json").read_text(encoding="utf-8"))
    helper = json.loads((out / "helper.json").read_text(encoding="utf-8"))
    w, h = geo["dimensions"]
    authored = geo["costs"]
    radius = geo["actor_radius"]
    ref = rows(out / "reference.logs.jsonl")
    info = [m for _, lvl, m, _ in ref if lvl == "info"]
    debug = [m for _, lvl, m, _ in ref if lvl == "debug"]
    layout = next(m for m in info if m["ev"] == "layout")
    for key in ("start", "goal", "door_a", "gate_b", "gauge"):
        assert layout[key] == geo[key], key
    dt = layout["dt"]

    # Grid at each tick: edits issued at tick t publish at t + 1.
    edits = {}
    for m in info:
        if m["ev"] in ("close_a", "close_b", "reopen"):
            edits[m["t"] + 1] = (tuple(m["cell"]), m["cost_written"])
    grids, costs = {}, list(authored)
    last_tick = debug[-1]["t"]
    for t in range(1, last_tick + 1):
        if t in edits:
            (c, r), value = edits[t]
            costs = list(costs)
            costs[r * w + c] = value
        grids[t] = costs

    report = {"set": str(out), "ticks": last_tick, "dt": dt}
    # Door closures never land on the actor.
    report["door_edits"] = [{"t": m["t"], "ev": m["ev"], "cell": m["cell"], "actor_from": m["actor_from"],
                             "actor_to": m["actor_to"]} for m in info if m["ev"] in ("close_a", "close_b", "reopen")]
    for e in report["door_edits"]:
        assert e["cell"] not in (e["actor_from"], e["actor_to"]) or e["ev"] == "reopen", e

    # Probes against the oracle.
    probes = {}
    for m in info:
        if m["ev"] != "probe":
            continue
        v, g = m["value"], grids[m["t"]]
        if "error" in v:
            probes[m["name"]] = {"error": v["error"]}
            continue
        diagonal = m["name"] != "four_way_start_goal"
        oracle = dijkstra(g, w, h, v["start"], v["end"], diagonal)
        if v.get("result", "path") is None:
            assert oracle is None, (m["name"], oracle)
            probes[m["name"]] = {"engine": None, "oracle": None}
            continue
        recomputed = path_cost(g, w, v["cells"], diagonal)
        assert v["cost"] == recomputed == oracle, (m["name"], v["cost"], recomputed, oracle)
        probes[m["name"]] = {"engine": v["cost"], "recomputed": recomputed, "oracle": oracle, "cells": v["cells"]}
    report["probes"] = probes

    # Route events against the oracle; remember the route in effect at each tick.
    routes, route_at, current = [], {}, None
    route_events = {m["t"]: m for m in info if m["ev"] == "route"}
    for t in range(1, last_tick + 1):
        if t in route_events:
            current = route_events[t]
            g = grids[t]
            oracle = dijkstra(g, w, h, current["anchor"], layout["goal"])
            if current["cells"] is None:
                assert oracle is None, (t, oracle)
            else:
                recomputed = path_cost(g, w, current["cells"])
                assert current["cost"] == recomputed == oracle, (t, current["cost"], recomputed, oracle)
                assert current["cells"][0] == current["anchor"] and current["cells"][-1] == layout["goal"]
            routes.append({"t": t, "anchor": current["anchor"], "cost": current["cost"],
                           "generation": current["generation"], "cells": current["cells"],
                           "oracle": oracle, "door_a": current["door_a"], "gate_b": current["gate_b"]})
        route_at[t] = current
    report["routes"] = routes

    # Per-tick costs (anchor = previous row's `to`) against the oracle.
    prev_to = layout["start"]
    per_tick_mismatch = 0
    for row in debug:
        if row["phase"] != "arrived" or row["cost"] is not None:
            oracle = dijkstra(grids[row["t"]], w, h, prev_to, layout["goal"])
            if row["cost"] != oracle:
                per_tick_mismatch += 1
        prev_to = row["to"]
    report["per_tick_cost_mismatches"] = per_tick_mismatch
    assert per_tick_mismatch == 0

    # Motion: on adjacent centre segments, steps taken from the route in effect.
    steps, max_disp, max_turn, min_clear, stationary = [], 0.0, 0.0, math.inf, {}
    sx, sz = world(layout["start"])
    prev = {"to": layout["start"], "pos": [sx, 0.0, sz], "yaw": 0.0}  # authored start, tick 0
    for row in debug:
        t, f, to, pos = row["t"], row["from"], row["to"], row["pos"]
        assert max(abs(f[0] - to[0]), abs(f[1] - to[1])) <= 1, (t, f, to)
        a, b = world(f), world(to)
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        frac = row["s"] / length if length else 0.0
        expect = (a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac)
        assert abs(expect[0] - pos[0]) < 1e-12 and abs(expect[1] - pos[2]) < 1e-12, (t, expect, pos)
        if f != to and abs(f[0] - to[0]) == 1 and abs(f[1] - to[1]) == 1:
            g = grids[t]
            assert g[f[1] * w + to[0]] > 0 and g[to[1] * w + f[0]] > 0, ("visible corner cut", t, f, to)
        min_clear = min(min_clear, clearance(grids[t], w, h, pos[0], pos[2]) - radius)
        if prev is not None:
            if to != prev["to"]:
                r = route_at[t]
                # The route queried this tick starts at the cell just reached.
                assert r is not None and r["cells"] is not None and r["cells"][0] == f and r["cells"][1] == to, (t, f, to, r and r["cells"])
                steps.append({"t": t, "from": f, "to": to, "route_t": r["t"]})
            disp = math.hypot(pos[0] - prev["pos"][0], pos[2] - prev["pos"][2])
            max_disp = max(max_disp, disp)
            turn = abs(math.atan2(math.sin(row["yaw"] - prev["yaw"]), math.cos(row["yaw"] - prev["yaw"])))
            max_turn = max(max_turn, turn)
            if disp == 0:
                key = row["phase"] if turn == 0 else f"{row['phase']}-turning"
                stationary[key] = stationary.get(key, 0) + 1
        prev = row
    assert max_disp <= SPEED * dt + EPS, max_disp
    assert max_turn <= TURN_RATE * dt + EPS, max_turn
    assert min_clear >= 0, min_clear
    report["motion"] = {"steps": len(steps), "max_displacement_per_tick_m": max_disp,
                        "speed_limit_per_tick_m": SPEED * dt, "max_yaw_change_per_tick_rad": max_turn,
                        "turn_limit_per_tick_rad": TURN_RATE * dt, "min_footprint_clearance_m": min_clear,
                        "stationary_ticks_by_phase": stationary, "step_log": steps}
    report["events"] = [m for m in info if m["ev"] not in ("route", "probe")]

    # Repeat and save/resume.
    report["repeat_logs_identical"] = (out / "reference.logs.jsonl").read_bytes() == (out / "repeat.logs.jsonl").read_bytes()
    report["repeat_state_identical"] = stdout_core(out / "reference.stdout.json") == stdout_core(out / "repeat.stdout.json")
    pre, post = rows(out / "resume.prefix.logs.jsonl"), rows(out / "resume.logs.jsonl")
    report["resume_save_tick"] = helper["resume_save_tick"]
    # `generation` is a session cache revision that restarts on save reopen; it
    # is compared separately and never used as identity.
    strip = lambda m: {k: v for k, v in m.items() if k != "generation"}
    joined = [(t, lvl, strip(m)) for t, lvl, m, _ in pre + post]
    report["resume_messages_identical_except_generation"] = joined == [(t, lvl, strip(m)) for t, lvl, m, _ in ref]
    generations = {}
    for a, b in zip(pre + post, ref):
        if "generation" in a[2] and a[2]["generation"] != b[2]["generation"]:
            key = f"{b[2]['generation']}->{a[2]['generation']}"
            generations.setdefault(key, [a[0], a[0]])[1] = a[0]
    report["resume_generation_differences"] = {k: {"first_tick": v[0], "last_tick": v[1]} for k, v in generations.items()}
    report["resume_raw_log_bytes_identical"] = (
        (out / "resume.prefix.logs.jsonl").read_bytes() + (out / "resume.logs.jsonl").read_bytes()
        == (out / "reference.logs.jsonl").read_bytes())
    elapsed = [abs(a[3]["elapsed_seconds"] - b[3]["elapsed_seconds"]) for a, b in zip(pre + post, ref)]
    report["resume_max_elapsed_seconds_difference"] = max(elapsed)
    report["resume_state_identical"] = stdout_core(out / "reference.stdout.json") == stdout_core(out / "resume.stdout.json")
    report["authored_unchanged"] = helper["authored_unchanged"]
    report["binary_sha256"] = sorted({p["binary_before"] for p in helper["plays"]} | {p["binary_after"] for p in helper["plays"]})
    report["failed_attempts"] = [{"name": p["name"], "failed": p["failed_attempts"]} for p in helper["plays"] if p["failed_attempts"]]
    for key in ("repeat_logs_identical", "repeat_state_identical", "resume_messages_identical_except_generation", "resume_state_identical",
                "authored_unchanged"):
        assert report[key], key

    # Frames: windows reopened from saves vs the full-course capture at the same tick.
    frames = {}
    for p in helper["plays"]:
        if p["camera"] is None:
            continue
        rep = json.loads((out / p["name"] / "report.json").read_text(encoding="utf-8"))
        frames[p["name"]] = {"camera": p["camera"], "start": p.get("start_tick", 0),
                             "shas": {f["tick"]: sha(out / p["name"] / f["file"]) for f in rep["frames"]}}
    cross = []
    for name, data in frames.items():
        if data["start"] == 0:
            continue
        for full, fdata in frames.items():
            if full == name or fdata["camera"] != data["camera"] or (fdata["start"] != 0 and full < name):
                continue
            shared = sorted(set(data["shas"]) & set(fdata["shas"]))
            same = sum(data["shas"][t] == fdata["shas"][t] for t in shared)
            cross.append({"window": name, "against": full, "shared_ticks": len(shared), "identical": same})
    cross = [c for c in cross if c["shared_ticks"]]
    report["window_frame_overlaps"] = cross
    for c in cross:
        assert c["shared_ticks"] == c["identical"], c
    report["frames"] = {k: {"camera": v["camera"], "start": v["start"], "count": len(v["shas"])} for k, v in frames.items()}
    return report, frames


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("set", type=Path)
    parser.add_argument("--compare", type=Path)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()
    report, frames = check_set(args.set)
    if args.compare:
        other, other_frames = check_set(args.compare)
        logs = all((args.set / f).read_bytes() == (args.compare / f).read_bytes()
                   for f in ("reference.logs.jsonl", "repeat.logs.jsonl", "resume.prefix.logs.jsonl", "resume.logs.jsonl"))
        # Both sets must contain the same captures, cameras and tick sets before
        # any frame hash is compared; a missing frame can never count as a match.
        keys_a, keys_b = sorted(frames), sorted(other_frames)
        if keys_a != keys_b:
            raise SystemExit(f"capture keys differ: {keys_a} != {keys_b}")
        for k in keys_a:
            if frames[k]["camera"] != other_frames[k]["camera"] or frames[k]["start"] != other_frames[k]["start"]:
                raise SystemExit(f"capture {k}: camera or start tick differs")
            if sorted(frames[k]["shas"]) != sorted(other_frames[k]["shas"]):
                raise SystemExit(f"capture {k}: tick sets differ")
        frame_total = sum(len(v["shas"]) for v in frames.values())
        frame_same = sum(frames[k]["shas"][t] == s for k in keys_a for t, s in other_frames[k]["shas"].items())
        core_a, aliases_a = normalized_core(args.set)
        core_b, aliases_b = normalized_core(args.compare)
        if aliases_a != aliases_b:
            raise SystemExit("generated-ID alias sets differ between projects")
        report["compare"] = {"other": str(args.compare), "logs_identical": logs,
                             "capture_keys_and_tick_sets_identical": True, "captures": len(keys_a),
                             "states_identical_raw": stdout_core(args.set / "reference.stdout.json") == stdout_core(args.compare / "reference.stdout.json"),
                             "states_identical_after_generated_id_mapping": core_a == core_b,
                             "generated_ids_mapped": len(aliases_a),
                             "generated_id_aliases": aliases_a,
                             "frames": frame_total, "frames_identical": frame_same,
                             "other_set_checks_passed": True}
        assert logs and frame_same == frame_total and core_a == core_b, report["compare"]
    text = json.dumps(report, indent=1)
    if args.json:
        if args.json.exists():
            raise SystemExit(f"{args.json} exists")
        args.json.write_text(text + "\n", encoding="utf-8")
    summary = {k: report[k] for k in ("ticks", "per_tick_cost_mismatches", "repeat_logs_identical", "repeat_state_identical",
                                      "resume_messages_identical_except_generation", "resume_generation_differences",
                                      "resume_raw_log_bytes_identical", "resume_max_elapsed_seconds_difference",
                                      "resume_state_identical", "resume_save_tick", "authored_unchanged")}
    summary["motion"] = {k: v for k, v in report["motion"].items() if k != "step_log"}
    summary["window_frame_overlaps"] = report["window_frame_overlaps"]
    if "compare" in report:
        summary["compare"] = report["compare"]
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
