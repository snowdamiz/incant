#!/usr/bin/env python3
"""Offline analysis of handoff 0067 evidence. CPU only; standard library only.

  python3 -I -B analyze.py <evidence-dir> <new-output.json>

1. Regression suites: every listed ignored test, its libtest outcome, counts.
2. Numeric cases: raw DRS-RECEIPT lines, verdict failures, budget vs ledger.
3. An independent Python replay of the natural-cadence adaptive run from the
   raw event log: witnessed dispatch, the single completion slot and the
   documented policy predict every receipt scale. Each consumed sample keeps
   its completion-latency interval [lo, hi] in ms: lo = witness dispatch -
   provider acquire (after the host's clock started), hi = next logged event
   after the dispatch - tick call (before the host's clock started).
4. Completion-latency bounds of every witnessed frame in every case.
5. Appearance: PNG decode and RGBA hash against the helper's own readback.
"""

import hashlib
import json
import re
import statistics
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

TEST_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)(?:, .*)?$")
SUMMARY = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; "
                     r"(\d+) filtered out")


def read_lines(path):
    return Path(path).read_text().splitlines() if Path(path).exists() else []


def suites(evidence, ledger):
    out = {}
    for entry in ledger:
        if entry["kind"] not in ("list", "suite"):
            continue
        lines = read_lines(evidence / "gpu-logs" / f"{entry['label']}.stdout.txt")
        if entry["kind"] == "list":
            names = [line[:-len(": test")] for line in lines if line.endswith(": test")]
            suite = entry["label"].replace("list-ignored-", "").replace("list-", "")
            key = "ignored" if entry["label"].startswith("list-ignored-") else "all"
            out.setdefault(suite, {})[f"listed_{key}"] = names
            continue
        suite = entry["label"].replace("suite-", "")
        results = {}
        for line in lines:
            match = TEST_LINE.match(line)
            if match:
                results[match.group(1)] = match.group(2)
        summary = [SUMMARY.match(line).groups() for line in lines if SUMMARY.match(line)]
        failures = []
        if "failures:" in lines:
            tail = lines[len(lines) - lines[::-1].index("failures:"):]
            failures = [line.strip() for line in tail if line.startswith("    ")]
        out.setdefault(suite, {}).update({
            "exit": entry["exit"], "timed_out": entry["timed_out"], "started": entry["started"],
            "ended": entry["ended"], "outcomes": results,
            "counts": {k: sum(1 for v in results.values() if v == k) for k in ("ok", "FAILED", "ignored")},
            "summary": summary[-1] if summary else None, "failures": failures})
    for suite, data in out.items():
        listed = set(data.get("listed_ignored", []))
        ran = set(data.get("outcomes", {}))
        data["ignored_listed"] = len(listed)
        data["listed_but_not_run"] = sorted(listed - ran) if "outcomes" in data else None
        data["run_but_not_listed"] = sorted(ran - listed) if "outcomes" in data else None
    return out


def receipts(path):
    out = []
    for line in read_lines(path):
        # libtest prints the first receipt on its "test <name> ... " line.
        if "DRS-RECEIPT " in line:
            out.append(json.loads(line.split("DRS-RECEIPT ", 1)[1]))
    return out


def classify(lo, hi, config):
    low, high = config["target_ms"] * (1 - config["deadband"]), config["target_ms"] * (1 + config["deadband"])
    if hi < low:
        return "Under"
    if lo > high:
        return "Over"
    if lo >= low and hi <= high:
        return "InBand"
    return "Ambiguous"


def admitted(lo, hi, config):
    low, high = config["target_ms"] * (1 - config["deadband"]), config["target_ms"] * (1 + config["deadband"])
    out = []
    if lo < low:
        out.append("Under")
    if hi >= low and lo <= high:
        out.append("InBand")
    if hi > high:
        out.append("Over")
    return out


def replay_all(events, probe, config):
    """Every receipt-scale sequence the witnessed deliveries admit (ambiguous intervals branch)."""
    acquire = {e[3]: e for e in events if e[1] == "acquire" and e[2] == probe}
    calls = {e[3]: e[0] for e in events if e[1] == "tick-call"}
    branches = [({"scale": config["initial"], "overload": 0, "recovery": 0, "cooldown": 0}, [])]
    slot, samples = ("idle", None), []
    for at, kind, name, frame, phase in events:
        if name != probe:
            continue
        if kind == "fire" and slot == ("pending", frame):
            slot = ("ready", frame)
        elif kind == "acquire":
            if slot[0] == "ready":
                k = slot[1]
                fire = next(i for i, e in enumerate(events) if e[1] == "fire" and e[2] == probe and e[3] == k)
                nxt = events[fire + 1][0] if fire + 1 < len(events) else float("inf")
                lo = (events[fire][0] - acquire[k][0]) / 1e6
                hi = (nxt - calls[acquire[k][4] // 10]) / 1e6
                classes = admitted(lo, hi, config)
                grown = []
                for state, predicted in branches:
                    for cls in classes:
                        nxt_state = step(dict(state), cls, config)
                        if (nxt_state, predicted) not in grown:
                            grown.append((nxt_state, list(predicted)))
                branches = grown
                samples.append({"frame": k, "consumed_before_frame": frame, "lo_ms": lo, "hi_ms": hi,
                                "admitted": classes})
                slot = ("idle", None)
            for state, predicted in branches:
                predicted.append(state["scale"])
        elif kind == "witness-registered" and slot[0] == "idle":
            slot = ("pending", frame)
    return [p for _, p in branches], samples


def step(state, kind, config):
    if state["cooldown"] > 0:
        state["cooldown"] -= 1
        return state
    if kind == "Over":
        state["recovery"] = 0
        state["overload"] += 1
        if state["overload"] >= config["overload_samples"]:
            state["overload"] = 0
            state = change(state, state["scale"] - config["slew"], config)
    elif kind == "Under":
        state["overload"] = 0
        state["recovery"] += 1
        if state["recovery"] >= config["recovery_samples"]:
            state["recovery"] = 0
            state = change(state, state["scale"] + config["slew"], config)
    else:
        state["overload"] = state["recovery"] = 0
    return state


def change(state, candidate, config):
    candidate = min(max(candidate, config["minimum"]), config["maximum"])
    if candidate != state["scale"]:
        state["scale"] = candidate
        state["cooldown"] = config["cooldown_samples"]
    return state


def replay(events, probe, config):
    """Independent re-implementation of the in-test replay (see module doc)."""
    state = {"scale": config["initial"], "overload": 0, "recovery": 0, "cooldown": 0}

    def observe(kind):
        if state["cooldown"] > 0:
            state["cooldown"] -= 1
            return
        if kind == "Over":
            state["recovery"] = 0
            state["overload"] += 1
            if state["overload"] >= config["overload_samples"]:
                state["overload"] = 0
                change(state["scale"] - config["slew"])
        elif kind == "Under":
            state["overload"] = 0
            state["recovery"] += 1
            if state["recovery"] >= config["recovery_samples"]:
                state["recovery"] = 0
                change(state["scale"] + config["slew"])
        else:
            state["overload"] = state["recovery"] = 0

    def change(candidate):
        candidate = min(max(candidate, config["minimum"]), config["maximum"])
        if candidate != state["scale"]:
            state["scale"] = candidate
            state["cooldown"] = config["cooldown_samples"]

    acquire = {e[3]: e for e in events if e[1] == "acquire" and e[2] == probe}
    calls = {e[3]: e[0] for e in events if e[1] == "tick-call"}
    slot, predicted, samples = ("idle", None), [], []
    for index, (at, kind, name, frame, phase) in enumerate(events):
        if name != probe:
            continue
        if kind == "fire" and slot == ("pending", frame):
            slot = ("ready", frame)
        elif kind == "acquire":
            if slot[0] == "ready":
                k = slot[1]
                fire = next(i for i, e in enumerate(events) if e[1] == "fire" and e[2] == probe and e[3] == k)
                nxt = events[fire + 1][0] if fire + 1 < len(events) else float("inf")
                lo = (events[fire][0] - acquire[k][0]) / 1e6
                hi = (nxt - calls[acquire[k][4] // 10]) / 1e6
                kind_ = classify(lo, hi, config)
                before = state["scale"]
                observe(kind_)
                samples.append({"frame": k, "consumed_before_frame": frame, "lo_ms": lo, "hi_ms": hi,
                                "class": kind_, "scale_before": before, "scale_after": state["scale"]})
                slot = ("idle", None)
            predicted.append(state["scale"])
        elif kind == "witness-registered" and slot[0] == "idle":
            slot = ("pending", frame)
    return predicted, samples


def latency_bounds(events):
    """[lo, hi] completion-latency bounds (ms) of every witnessed frame. lo starts at the
    frame's provider acquire; hi starts at the last tick call before that acquire and
    ends at the first logged event after the dispatch (event order, so probes that
    reuse tick numbers within one case cannot alias)."""
    out = []
    for i, e in enumerate(events):
        if e[1] != "fire":
            continue
        acquire = next((j for j in range(i - 1, -1, -1)
                        if events[j][1] == "acquire" and events[j][2] == e[2] and events[j][3] == e[3]), None)
        if acquire is None:
            continue
        call = next((j for j in range(acquire - 1, -1, -1) if events[j][1] == "tick-call"), None)
        nxt = events[i + 1][0] if i + 1 < len(events) else None
        out.append({"probe": e[2], "frame": e[3], "dispatch_phase": e[4],
                    "lo_ms": (e[0] - events[acquire][0]) / 1e6,
                    "hi_ms": (nxt - events[call][0]) / 1e6 if nxt is not None and call is not None else None})
    return out


def summarize(values):
    values = [v for v in values if v is not None]
    if not values:
        return None
    return {"n": len(values), "min": min(values), "median": statistics.median(values), "max": max(values)}


def cases(evidence, ledger):
    out = {}
    for entry in ledger:
        if entry["kind"] != "case":
            continue
        found = receipts(evidence / "gpu-logs" / f"{entry['label']}.stdout.txt")
        by_kind = {}
        for r in found:
            by_kind.setdefault(r["kind"], []).append(r["body"])
        events = by_kind.get("events", [[]])[-1]
        record = {"exit": entry["exit"], "timed_out": entry["timed_out"], "receipts": len(found),
                  "verdict": by_kind.get("verdict", [None])[-1], "budget": by_kind.get("budget", [None])[-1],
                  "attempts_reserved": entry["attempts_reserved"], "adapter": by_kind.get("adapter", [None])[-1],
                  "latency_bounds": latency_bounds(events)}
        record["latency_summary"] = {"lo_ms": summarize([b["lo_ms"] for b in record["latency_bounds"]]),
                                     "hi_ms": summarize([b["hi_ms"] for b in record["latency_bounds"]])}
        probes = {"adaptive_natural_cadence": "A", "adaptive_paced_explicit_dispatch": "P"}
        if entry["case"] in probes:
            config = by_kind["fixture"][-1]["config"]
            rust = by_kind["replay"][-1]
            admitted_sequences, samples = replay_all(events, probes[entry["case"]], config)
            rust_sets = rust["predicted_scale16"]
            rust_sets = rust_sets if rust_sets and isinstance(rust_sets[0], list) else [rust_sets]
            record["independent_replay"] = {
                "admitted_scale16_sequences": admitted_sequences, "observed_scale16": rust["observed_scale16"],
                "rust_admitted_scale16_sequences": rust_sets, "samples": samples,
                "observed_admitted": rust["observed_scale16"] in admitted_sequences,
                "matches_rust_replay": sorted(admitted_sequences) == sorted(rust_sets),
                "ambiguous_samples": sum(1 for s in samples if len(s["admitted"]) > 1)}
        out[entry["case"]] = record
    return out


def physical(box, region, viewport):
    sx, sy = region[2] / viewport[0], region[3] / viewport[1]
    return [region[0] + box[0] * sx, region[1] + box[1] * sy, region[0] + box[2] * sx, region[1] + box[3] * sy]


def rounded_distance(cx, cy, rect, radii):
    import math
    x, y, w, h = rect
    hx, hy = w / 2, h / 2
    px, py = cx - x - hx, cy - y - hy
    top, bottom = (radii[1], radii[2]) if px > 0 else (radii[0], radii[3])
    radius = min(bottom if py > 0 else top, min(hx, hy))
    qx, qy = abs(px) - hx + radius, abs(py) - hy + radius
    return math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - radius


def offline_frames(evidence):
    """Independent checks on retained frames: run 1's opaque UI interiors in physical
    pixels, and nearest-texel consistency (every output pixel group that maps to one
    internal texel is uniform) outside UI boxes and viewport-edge bands."""
    directory = evidence / "captures" / "appearance"
    report = json.loads((directory / "report.json").read_text())
    frames = {}
    for frame in report["frames"]:
        w, h, rows = pngio.read(directory / frame["png"])
        frames[frame["name"]] = (w, h, rows, frame)
    out = {}
    disabled = frames["a1-offset-ui-disabled"]
    ui = disabled[3]["state"]["ui"]
    region = disabled[3]["viewport"]["rect"]
    radii = disabled[3]["viewport"]["corner_radii"]
    regions = []
    for widget in ui["widgets"]:
        if widget["role"] in ("Button", "TextInput"):
            b = physical(widget["visible"], region, ui["viewport"])
            regions.append([int(-(-b[0] // 1)) + 1, int(-(-b[1] // 1)) + 1, int(b[2] // 1) - 1, int(b[3] // 1) - 1])
    for name in ("a3-offset-ui-75", "a4-offset-ui-50"):
        rows = frames[name][2]
        compared = mismatched = 0
        for r in regions:
            for y in range(r[1], r[3]):
                for x in range(r[0], r[2]):
                    near_x = x < r[0] + 5 or x + 5 >= r[2]
                    near_y = y < r[1] + 5 or y + 5 >= r[3]
                    if near_x and near_y:
                        continue
                    compared += 1
                    mismatched += disabled[2][y][4 * x:4 * x + 4] != rows[y][4 * x:4 * x + 4]
        out[f"opaque_interiors_{name}_vs_disabled"] = {"compared": compared, "mismatched": mismatched,
                                                       "regions": len(regions)}
    for name, (w, h, rows, frame) in frames.items():
        receipt = frame["state"]["receipt"]
        if not receipt or receipt["scale16"] == 16 or receipt["pixel_perfect"]:
            continue
        n = receipt["scale16"]
        view = frame["viewport"]["rect"] if frame["viewport"] else [0, 0, w, h]
        view_radii = frame["viewport"]["corner_radii"] if frame["viewport"] else [0, 0, 0, 0]
        ui_frame = frame["state"]["ui"]
        boxes = [physical(wd["visible"], view, ui_frame["viewport"]) for wd in ui_frame["widgets"]] if ui_frame else []
        groups, broken, checked = {}, 0, 0
        for y in range(h):
            ty = ((2 * y + 1) * n) // 32
            for x in range(w):
                cx, cy = x + 0.5, y + 0.5
                if any(b[0] - 1 <= cx < b[2] + 1 and b[1] - 1 <= cy < b[3] + 1 for b in boxes):
                    continue
                if frame["viewport"] and rounded_distance(cx, cy, view, view_radii) > -0.75:
                    continue
                tx = ((2 * x + 1) * n) // 32
                pixel = rows[y][4 * x:4 * x + 4]
                key = (tx, ty)
                checked += 1
                if key in groups:
                    broken += groups[key] != pixel
                else:
                    groups[key] = pixel
        out[f"nearest_groups_{name}"] = {"scale16": n, "pixels_checked": checked, "texel_groups": len(groups),
                                         "pixels_differing_within_group": broken}
    return out


def appearance(evidence, ledger):
    entries = [e for e in ledger if e["kind"] == "appearance"]
    if not entries:
        return None
    directory = evidence / "captures" / "appearance"
    report = json.loads((directory / "report.json").read_text())
    frames = {}
    for frame in report["frames"]:
        path = directory / frame["png"]
        width, height, rows = pngio.read(path)
        rgba = b"".join(rows)
        frames[frame["name"]] = {
            "size": [width, height], "within_960x540": width <= 960 and height <= 540,
            "decoded_rgba_matches_readback": hashlib.sha256(rgba).hexdigest() == frame["rgba_sha256"],
            "png_sha256_matches": hashlib.sha256(path.read_bytes()).hexdigest() == frame["png_sha256"],
            "receipt": frame["state"]["receipt"], "requested": frame["requested"]}
    return {"ledger": {k: entries[-1][k] for k in ("exit", "timed_out", "renders_reserved", "started", "ended")},
            "adapter": report["adapter"], "renders": report["renders"], "frames": frames}


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, output = Path(sys.argv[1]), Path(sys.argv[2])
    ledger = [json.loads(line) for line in read_lines(evidence / "gpu-ledger.jsonl") if line.strip()]
    result = {"ledger_sha256": hashlib.sha256((evidence / "gpu-ledger.jsonl").read_bytes()).hexdigest(),
              "children": len(ledger),
              "numeric_attempts_reserved": sum(e.get("attempts_reserved", 0) for e in ledger),
              "appearance_renders_reserved": sum(e.get("renders_reserved", 0) for e in ledger),
              "suites": suites(evidence, ledger), "cases": cases(evidence, ledger),
              "appearance": appearance(evidence, ledger), "offline_frames": offline_frames(evidence)}
    with open(output, "x") as out:
        json.dump(result, out, indent=1)
        out.write("\n")
    print(json.dumps({"children": result["children"], "attempts": result["numeric_attempts_reserved"],
                      "renders": result["appearance_renders_reserved"],
                      "suites": {k: (v.get("counts"), v.get("ignored_listed"), v.get("listed_but_not_run"))
                                 for k, v in result["suites"].items()},
                      "cases": {k: (v["exit"], (v["verdict"] or {}).get("failures")) for k, v in result["cases"].items()},
                      "replays": {k: v.get("independent_replay", {}).get("observed_admitted")
                                  for k, v in result["cases"].items() if "independent_replay" in v},
                      "offline_frames": result["offline_frames"]},
                     indent=1))


if __name__ == "__main__":
    main()
