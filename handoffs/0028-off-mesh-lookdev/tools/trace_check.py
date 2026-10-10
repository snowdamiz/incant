#!/usr/bin/env python3
"""Independent checks of actual off-mesh traversal motion for handoff 0028.

Reads only engine outputs written by off_mesh_course.py: geometry.json, the
committed script logs (`--log-output`), play stdout JSON and PNG frames. Every
number is reported unrounded (Python repr of the logged double).

  summary RUN_DIR [--json OUT]   metadata-vs-motion, snapped endpoints, grounding,
                                 launch/landing continuity, pauses, yaw steps,
                                 logical capsule clearance, rebuild reports
  resume RUN_DIR                 reference vs save-prefix + reopened logs/state
  window RUN_DIR FULL RESUMED    frames and logs of two capture windows over the
                                 same ticks (uninterrupted vs reopened mid-jump)
  compare DIR_A DIR_B            independent repeats: every log, stdout state and
                                 frame byte-compared
"""

import argparse
import hashlib
import json
import math
from pathlib import Path

HZ = 60


def read_logs(path):
    rows, events = {}, []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        message = json.loads(entry["message"])
        if entry["level"] == "debug":
            rows[message["t"]] = message
        else:
            events.append(message)
    return rows, events


def parabola(a, b, t, apex=0.35):
    h = apex + abs(b[1] - a[1]) / 2
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t + 4 * h * t * (1 - t), a[2] + (b[2] - a[2]) * t]


def platform_under(geometry, p):
    for name, (x0, x1, z0, z1, top) in geometry["platforms"].items():
        if x0 <= p[0] <= x1 and z0 <= p[2] <= z1:
            return name, top
    return None, None


def segment_box_distance_2d(a, b, box):
    """Distance in the (x, y) plane between segment a-b and the axis-aligned box
    (x0, x1, y0, y1); 0 when they intersect. Sampled densely then refined."""
    def point_box(p):
        dx = max(box[0] - p[0], 0, p[0] - box[1])
        dy = max(box[2] - p[1], 0, p[1] - box[3])
        return math.hypot(dx, dy)
    best = math.inf
    for i in range(201):
        t = i / 200
        best = min(best, point_box((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)))
    return best


def summary(run_dir, json_out=None):
    run_dir = Path(run_dir)
    geometry = json.loads((run_dir / "geometry.json").read_text(encoding="utf-8"))
    rows, events = read_logs(run_dir / "reference.logs.jsonl")
    ticks = sorted(rows)
    radius, height = geometry["radius"], geometry["height"]
    authored = {link["id"]: link for link in geometry["links"]}
    plans = [e for e in events if e["ev"] == "plan"]
    out = {"plans": [], "endpoints": [], "jumps": [], "grounding": {}, "continuity": {}, "pauses": [],
           "metadata_mismatch": [], "parabola_max_error": 0.0, "min_segment": None, "reports": {}}

    # Routes, null queries and snapped vs authored endpoints.
    routes = []
    for plan in plans:
        path = plan["path"]
        out["plans"].append({"t": plan["t"], "name": plan["name"], "null": path is None,
                             "generation": None if path is None else path["generation"],
                             "traversals": None if path is None else path["traversals"]})
        if path is None:
            continue
        points = path["points"]
        if plan["name"] in ("out", "back-after-unlock"):
            routes.append((plan["t"], points, path["traversals"]))
        for i in range(len(points) - 1):
            d = math.dist(points[i], points[i + 1])
            if out["min_segment"] is None or d < out["min_segment"]["length"]:
                out["min_segment"] = {"plan": plan["name"], "index": i, "length": d, "a": points[i], "b": points[i + 1]}
        for t in path["traversals"]:
            link = authored[t["link_id"]]
            a_auth, b_auth = (link["end"], link["start"]) if t["reversed"] else (link["start"], link["end"])
            a, b = points[t["from_index"]], points[t["to_index"]]
            out["endpoints"].append({"plan": plan["name"], "link": t["link_id"], "reversed": t["reversed"],
                                     "from_index": t["from_index"], "to_index": t["to_index"],
                                     "launch": a, "launch_authored": a_auth, "launch_delta": [a[i] - a_auth[i] for i in range(3)],
                                     "landing": b, "landing_authored": b_auth, "landing_delta": [b[i] - b_auth[i] for i in range(3)]})

    # Metadata vs motion: for each logged tick find the active route and check
    # that m == 'jump' exactly when the segment is a traversal from_index.
    def route_at(t):
        active = None
        for start, points, traversals in routes:
            if start <= t:
                active = (points, traversals)
        return active
    launches = [e for e in events if e["ev"] == "launch"]
    lands = [e for e in events if e["ev"] == "land"]
    for t in ticks:
        row = rows[t]
        if row["ph"] not in ("out", "return"):
            continue
        points, traversals = route_at(t)
        is_link = any(tr["from_index"] == row["seg"] and tr["to_index"] == row["seg"] + 1 for tr in traversals)
        if (row["m"] == "jump") != (is_link and row["seg"] < len(points) - 1):
            out["metadata_mismatch"].append({"t": t, "m": row["m"], "seg": row["seg"]})
        if row["m"] == "jump":
            expected = parabola(points[row["seg"]], points[row["seg"] + 1], row["s"])
            err = max(abs(expected[i] - row["p"][i]) for i in range(3))
            out["parabola_max_error"] = max(out["parabola_max_error"], err)

    # Per-jump summary from the motion itself.
    platforms = geometry["platforms"]
    for launch, land in zip(launches, lands):
        air = [t for t in ticks if launch["t"] <= t < land["t"] and rows[t]["m"] == "jump"]
        a, b = launch["from"], launch["to"]
        peak = max(rows[t]["p"][1] for t in air)
        # Logical clearance of the capsule core (feet + r .. feet + h - r) from each
        # platform's (x, y) profile; links run along x at constant z, inside every
        # platform's z extent, so the 2-D profile is exact for these boxes.
        clearance = math.inf
        worst = None
        for t in air:
            p = rows[t]["p"]
            core = ((p[0], p[1] + radius), (p[0], p[1] + height - radius))
            for name, (x0, x1, z0, z1, top) in platforms.items():
                d = segment_box_distance_2d(core[0], core[1], (x0, x1, -10.0, top)) - radius
                if d < clearance:
                    clearance, worst = d, {"t": t, "platform": name, "feet": p}
        before = rows[launch["t"] - 1]["p"]
        first = rows[launch["t"]]["p"]
        last_air = rows[air[-1]]["p"]
        landed = rows[land["t"]]["p"]
        after = rows[land["t"] + 1]["p"]
        out["jumps"].append({
            "link": launch["link"], "key": launch["key"], "reversed": launch["reversed"],
            "launch_tick": launch["t"], "land_tick": land["t"], "airborne_ticks": len(air),
            "duration_s": launch["duration_s"], "peak_feet_y": peak, "peak_above_launch": peak - a[1],
            "peak_above_landing": peak - b[1], "min_capsule_clearance": clearance, "clearance_at": worst,
            "speed_before_launch": math.dist(before, rows[launch["t"] - 2]["p"]) * HZ,
            "speed_launch_tick": math.dist(first, before) * HZ,
            "speed_last_airborne": math.dist(last_air, rows[air[-1] - 1]["p"]) * HZ,
            "speed_landing_tick": math.dist(landed, last_air) * HZ,
            "speed_after_landing": math.dist(after, landed) * HZ,
            "vertical_speed_into_landing": (landed[1] - last_air[1]) * HZ,
            "landing_feet": landed, "landing_target": b,
        })

    # Grounding: feet height minus true platform top on walking ticks.
    offsets = {}
    for t in ticks:
        row = rows[t]
        if row["m"] != "walk":
            continue
        name, top = platform_under(geometry, row["p"])
        offsets.setdefault(name, set()).add(row["p"][1] - top if top is not None else None)
    out["grounding"] = {k: sorted(v) for k, v in offsets.items()}

    # Continuity and pauses while a leg is active.
    steps, zero = [], []
    for prev, t in zip(ticks, ticks[1:]):
        if rows[t]["ph"] not in ("out", "return"):
            continue
        d = math.dist(rows[t]["p"], rows[prev]["p"])
        steps.append((d * HZ, t))
        if d < 1e-9:
            zero.append(t)
    walk_speeds = [s for s, t in steps if rows[t]["m"] == "walk" and rows[t - 1]["m"] == "walk"
                   and all(abs(t - j["launch_tick"]) > 1 and abs(t - j["land_tick"]) > 1 for j in out["jumps"])]
    out["continuity"] = {
        "max_tick_speed": max(steps), "min_tick_speed_moving": min(steps),
        "walk_speed_min": min(walk_speeds), "walk_speed_max": max(walk_speeds),
        "zero_motion_ticks_in_active_legs": zero,
        "max_yaw_step_deg": max(abs(math.degrees(rows[t]["yaw"] - rows[p]["yaw"])) for p, t in zip(ticks, ticks[1:])),
    }
    arrive = [e["t"] for e in events if e["ev"] == "arrive"]
    out["pauses"] = {"arrive_ticks": arrive, "deliberate_pause_ticks": [arrive[0] + 1, next(
        e["t"] for e in events if e["ev"] == "enable")]}

    # Rebuild reports from the stdout of the short log-only runs.
    for path in sorted(run_dir.glob("report-t*.stdout.json")):
        state = json.loads(path.read_text(encoding="utf-8"))["state"]
        for _, report in state["navigation"].items():
            out["reports"][state["tick"]] = {"generation": report["generation"], "active_links": report["active_links"],
                                             "rebuilt": len(report["rebuilt"]), "reused": len(report["reused"]),
                                             "removed": len(report["removed"]), "polygons": report["polygons"]}
    text = json.dumps(out, indent=1)
    if json_out:
        Path(json_out).write_text(text + "\n", encoding="utf-8")
    print(text)


def strip_generation(value):
    if isinstance(value, dict):
        return {k: strip_generation(v) for k, v in value.items() if k not in ("generation", "generations")}
    if isinstance(value, list):
        return [strip_generation(v) for v in value]
    return value


def resume(run_dir):
    run_dir = Path(run_dir)
    lines = lambda name: (run_dir / f"{name}.logs.jsonl").read_text(encoding="utf-8").splitlines()
    ref = lines("reference")
    pre, res = lines("resume.prefix"), lines("resume")
    both = pre + res
    stdout = lambda name: json.loads((run_dir / f"{name}.stdout.json").read_text(encoding="utf-8"))
    r, s, p = stdout("reference"), stdout("resume"), stdout("resume.prefix")
    diffs = [(json.loads(a)["tick"], json.loads(a)["message"], json.loads(b)["message"]) for a, b in zip(ref, both) if a != b]
    generation_only = all(strip_generation(json.loads(x)) == strip_generation(json.loads(y)) for _, x, y in diffs)
    result = {
        "save_tick": p["state"]["tick"], "reopen_start_tick": s["start_tick"],
        "mid_state": {k: p["script_state"][k] for k in ("phase", "seg", "s", "pos")},
        "log_lines": [len(ref), len(pre), len(res)], "log_lines_equal": len(ref) == len(both),
        "differing_log_lines": len(diffs), "differences_generation_only": generation_only,
        "differences": [{"tick": t, "reference": json.loads(a).get("path", {}).get("generation"),
                         "reopened": json.loads(b).get("path", {}).get("generation"), "event": json.loads(a)["name"]}
                        for t, a, b in diffs],
        "final_entities_equal": r["state"]["entities"] == s["state"]["entities"],
        "script_state_equal": r["script_state"] == s["script_state"],
        "script_state_equal_except_generation": strip_generation(r["script_state"]) == strip_generation(s["script_state"]),
        "generations": [r["script_state"]["generations"], s["script_state"]["generations"]],
        "reopened_navigation": {k: {"generation": v["generation"], "active_links": v["active_links"],
                                    "rebuilt": len(v["rebuilt"]), "reused": len(v["reused"])}
                                for k, v in s["state"]["navigation"].items()},
        "reference_navigation": {k: {"generation": v["generation"], "active_links": v["active_links"]}
                                 for k, v in r["state"]["navigation"].items()},
    }
    print(json.dumps(result, indent=1))
    return result


def frames(directory):
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path(directory).glob("*.png"))}


def window(run_dir, full, resumed):
    run_dir = Path(run_dir)
    report = lambda name: json.loads((run_dir / name / "report.json").read_text(encoding="utf-8"))
    fr, rr = report(full), report(resumed)
    tick_of = lambda rep: {f["tick"] if isinstance(f, dict) and "tick" in f else None: f for f in rep.get("frames", [])}
    full_frames, resumed_frames = frames(run_dir / full), frames(run_dir / resumed)
    full_logs = {json.loads(l)["tick"]: l for l in (run_dir / f"{full}.logs.jsonl").read_text(encoding="utf-8").splitlines()
                 if json.loads(l)["level"] == "debug"}
    resumed_logs = {json.loads(l)["tick"]: l for l in (run_dir / f"{resumed}.logs.jsonl").read_text(encoding="utf-8").splitlines()
                    if json.loads(l)["level"] == "debug"}
    shared_log_ticks = sorted(set(full_logs) & set(resumed_logs))
    full_start = json.loads((run_dir / f"{full}.stdout.json").read_text(encoding="utf-8"))["start_tick"]
    resumed_start = json.loads((run_dir / f"{resumed}.stdout.json").read_text(encoding="utf-8"))["start_tick"]
    # Frame files are named by absolute simulation tick (frame-000801.png).
    def absolute(names):
        return {int(n.split("-")[1].split(".")[0]): h for n, h in names.items()}
    fa, ra = absolute(full_frames), absolute(resumed_frames)
    shared = sorted(set(fa) & set(ra))
    mismatched = [t for t in shared if fa[t] != ra[t]]
    result = {"full_start": full_start, "resumed_start": resumed_start, "shared_frame_ticks": [shared[0], shared[-1]] if shared else None,
              "shared_frames": len(shared), "mismatched_frames": mismatched,
              "shared_log_ticks": len(shared_log_ticks),
              "mismatched_log_ticks": [t for t in shared_log_ticks if full_logs[t] != resumed_logs[t]],
              "frame_naming": "absolute tick"}
    _ = (tick_of, fr, rr)
    print(json.dumps(result, indent=1))
    return result


def random_ids(run_dir):
    """Map the IDs that `init`/`import` mint randomly to stable labels."""
    project = json.loads((run_dir / "offmesh.incant.json").read_text(encoding="utf-8"))
    labels = {sid: "<scene>" for sid in project["scenes"]}
    for aid, asset in project["assets"].items():
        labels[aid] = f"<asset:{asset['path']}:{asset['sha256']}>"
    return labels


def normalize(value, labels):
    if isinstance(value, dict):
        return {labels.get(k, k): normalize(v, labels) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize(v, labels) for v in value]
    if isinstance(value, str):
        return labels.get(value, value)
    return value


def compare(dir_a, dir_b):
    a, b = Path(dir_a), Path(dir_b)
    result = {"logs": {}, "frames": {}, "states": {}, "states_raw_equal": {},
              "normalized": "scene ULID and imported asset ULIDs (random per init/import) replaced by stable labels"}
    ids_a, ids_b = random_ids(a), random_ids(b)
    for path in sorted(a.glob("*.logs.jsonl")):
        other = b / path.name
        result["logs"][path.name] = other.exists() and path.read_bytes() == other.read_bytes()
    for path in sorted(a.glob("*.stdout.json")):
        other = b / path.name
        if not other.exists():
            result["states"][path.name] = False
            continue
        x, y = json.loads(path.read_text(encoding="utf-8")), json.loads(other.read_text(encoding="utf-8"))
        raw = x["state"] == y["state"] and x["script_state"] == y["script_state"]
        same = (normalize(x["state"], ids_a) == normalize(y["state"], ids_b)
                and normalize(x["script_state"], ids_a) == normalize(y["script_state"], ids_b))
        result["states"][path.name] = same
        result["states_raw_equal"][path.name] = raw
    for directory in sorted(p for p in a.iterdir() if p.is_dir() and (p / "report.json").exists()):
        fa, fb = frames(directory), frames(b / directory.name)
        result["frames"][directory.name] = {"count": len(fa), "identical": fa == fb,
                                            "mismatched": [k for k in fa if fa.get(k) != fb.get(k)]}
    result["all_logs_identical"] = all(result["logs"].values())
    result["all_states_identical"] = all(result["states"].values())
    result["all_frames_identical"] = all(v["identical"] for v in result["frames"].values())
    result["total_frames"] = sum(v["count"] for v in result["frames"].values())
    print(json.dumps(result, indent=1))
    return result


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("summary")
    s.add_argument("run_dir")
    s.add_argument("--json")
    r = sub.add_parser("resume")
    r.add_argument("run_dir")
    w = sub.add_parser("window")
    w.add_argument("run_dir")
    w.add_argument("full")
    w.add_argument("resumed")
    c = sub.add_parser("compare")
    c.add_argument("a")
    c.add_argument("b")
    args = parser.parse_args()
    if args.cmd == "summary":
        summary(args.run_dir, args.json)
    elif args.cmd == "resume":
        resume(args.run_dir)
    elif args.cmd == "window":
        window(args.run_dir, args.full, args.resumed)
    else:
        compare(args.a, args.b)


if __name__ == "__main__":
    main()
