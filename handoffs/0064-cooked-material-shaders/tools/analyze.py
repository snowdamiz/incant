#!/usr/bin/env python3
"""CPU analysis of handoff 0064 review captures and GPU test logs. Standard library only.

  python3 -I -B analyze.py <evidence-dir> <new-output.json>

Reads captures/<variant>/<plan>/report.json and PNGs, gpu-ledger.jsonl and the
libtest logs; never starts a renderer. Every comparison is on unmasked whole
frames: PNG bytes, decoded RGBA SHA-256 (checked against the helper's own
readback hash) and per-row hashes. Pixel-level statistics are computed for
every frame that exists as a PNG in both compared runs.
"""

import hashlib
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(evidence, variant, plan):
    directory = evidence / "captures" / variant / plan
    report = json.loads((directory / "report.json").read_text())
    seen = {}
    for frame in report["frames"]:
        base = f"{frame['step']}@{frame['tick']}:{frame['width']}x{frame['height']}"
        seen[base] = seen.get(base, -1) + 1
        frame["key"] = f"{base}#{seen[base]}"  # occurrence within the run, normally 0
    return directory, report


def decode(path):
    width, height, rows = pngio.read(path)
    return width, height, rows, hashlib.sha256(b"".join(rows)).hexdigest()


def pixel_diff(rows_a, rows_b, width):
    differing, over_one, worst, bbox = 0, 0, 0, [None, None, None, None]
    for y, (ra, rb) in enumerate(zip(rows_a, rows_b)):
        if ra == rb:
            continue
        for x in range(width):
            pa, pb = ra[4 * x:4 * x + 4], rb[4 * x:4 * x + 4]
            if pa != pb:
                delta = max(abs(p - q) for p, q in zip(pa, pb))
                differing += 1
                over_one += delta > 1
                worst = max(worst, delta)
                bbox = [x if bbox[0] is None else min(bbox[0], x), y if bbox[1] is None else min(bbox[1], y),
                        x if bbox[2] is None else max(bbox[2], x), y if bbox[3] is None else max(bbox[3], y)]
    return {"differing_pixels": differing, "pixels_over_one": over_one, "max_channel_delta": worst,
            "bbox": bbox if differing else None}


def compare_runs(evidence, a, b):
    """Frame-by-frame comparison of two runs of the same plan (or a frame subset by key)."""
    (dir_a, rep_a), (dir_b, rep_b) = load(evidence, *a), load(evidence, *b)
    frames_b = {f["key"]: f for f in rep_b["frames"]}
    rows = []
    for fa in rep_a["frames"]:
        fb = frames_b.get(fa["key"])
        if fb is None:
            continue
        entry = {"frame": fa["key"], "rgba_sha256_a": fa["rgba_sha256"], "rgba_sha256_b": fb["rgba_sha256"],
                 "identical": fa["rgba_sha256"] == fb["rgba_sha256"],
                 "differing_rows": sum(x != y for x, y in zip(fa["row_hashes"], fb["row_hashes"]))}
        if "png" in fa and "png" in fb:
            pa, pb = dir_a / fa["png"]["name"], dir_b / fb["png"]["name"]
            wa, ha, rows_a, da = decode(pa)
            wb, hb, rows_b, db = decode(pb)
            entry["png"] = {"a": {"name": pa.name, "sha256": sha(pa), "decoded_rgba_sha256": da,
                                  "matches_readback": da == fa["rgba_sha256"], "chunks": pngio.chunk_types(pa.read_bytes())},
                            "b": {"name": pb.name, "sha256": sha(pb), "decoded_rgba_sha256": db,
                                  "matches_readback": db == fb["rgba_sha256"]},
                            "bytes_identical": sha(pa) == sha(pb)}
            entry["pixels"] = pixel_diff(rows_a, rows_b, wa) if (wa, ha) == (wb, hb) else {"error": "size"}
        rows.append(entry)
    return {"a": "/".join(a), "b": "/".join(b), "frames_compared": len(rows),
            "identical": sum(r["identical"] for r in rows), "frames": rows}


def frame_by(report, step, tick, size, nth=0):
    found = [f for f in report["frames"] if f["step"] == step and f["tick"] == tick
             and (f["width"], f["height"]) == size]
    return found[nth] if len(found) > nth else None


def within_run(evidence, variant, plan="main"):
    directory, report = load(evidence, variant, plan)
    checks = {}

    def same(name, x, y):
        checks[name] = {"a": x["key"], "b": y["key"], "identical": x["rgba_sha256"] == y["rgba_sha256"],
                        "differing_rows": sum(p != q for p, q in zip(x["row_hashes"], y["row_hashes"]))}

    wide = (960, 540)
    same("M-wide repeated after L/Z/R resource churn", frame_by(report, "M-wide", 0, wide),
         frame_by(report, "M-wide-again", 0, wide))
    same("R-v1 rendered again after R-v2 was prepared", frame_by(report, "R-v1", 0, (480, 270)),
         frame_by(report, "R-v1-again", 0, (480, 270)))
    # Reimport must change the panel only; the unchanged sphere keeps its pixels.
    v1, v2 = frame_by(report, "R-v1", 0, (480, 270)), frame_by(report, "R-v2", 0, (480, 270))
    if "png" in v1 and "png" in v2:
        _, _, rows1, _ = decode(directory / v1["png"]["name"])
        _, _, rows2, _ = decode(directory / v2["png"]["name"])
        checks["R-v1 versus R-v2 (reimported panel image)"] = pixel_diff(rows1, rows2, 480)
    lights = [f for f in report["frames"] if f["step"] == "L-persp"]
    checks["L-persp sequence"] = [{"frame": f["key"], "update": f["update"], "prepared": f["retained_scenes_prepared"],
                                   "lights": f["lights"], "rgba_sha256": f["rgba_sha256"]} for f in lights]
    consecutive = []
    for x, y in zip(lights, lights[1:]):
        if (x["width"], x["height"]) == (y["width"], y["height"]):
            consecutive.append({"from": x["key"], "to": y["key"],
                                "differing_rows": sum(p != q for p, q in zip(x["row_hashes"], y["row_hashes"]))})
    checks["L-persp consecutive same-size frames differ"] = consecutive
    controls = []
    for f in report["frames"]:
        if "controls" in f:
            controls.append({"frame": f["key"], **{name: {k: v for k, v in value.items() if k != "rgba_sha256"}
                                                   for name, value in f["controls"].items()}})
    checks["controls"] = controls
    checks["controls_summary"] = {
        name: {"frames": sum(1 for c in controls if name in c),
               "exact": sum(1 for c in controls if c.get(name, {}).get("differing_pixels") == 0),
               "worst_delta": max((c[name]["max_channel_delta"] for c in controls if name in c), default=None),
               "worst_pixels": max((c[name]["differing_pixels"] for c in controls if name in c), default=None)}
        for name in ("retained_repeat", "nonretained_clustered", "nonretained_all_lights")}
    checks["adapter"] = report["adapter"]
    checks["pngs_written"] = report["pngs_written"]
    checks["seconds"] = report["seconds"]
    checks["black_or_flat_frames"] = [f["key"] for f in report["frames"]
                                      if f["black_pixels"] == f["width"] * f["height"]]
    return checks


def libtest(evidence):
    out = {}
    ledger = [json.loads(line) for line in (evidence / "gpu-ledger.jsonl").read_text().splitlines() if line.strip()]
    for entry in ledger:
        if entry["kind"] != "test":
            continue
        text = (evidence / "gpu-logs" / f"{entry['label']}.stdout.txt").read_text()
        summary = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; "
                             r"(\d+) filtered out; finished in ([\d.]+)s", text, re.M)
        tests = re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", text, re.M)
        failures = re.search(r"^failures:\n\n?((?:    \S+\n)+)", text, re.M)
        out[entry["label"]] = {"exit": entry["exit"], "timed_out": entry["timed_out"], "pid": entry["pid"],
                               "started": entry["started"], "ended": entry["ended"], "summary": summary,
                               "ok_lines": sum(1 for _, r in tests if r == "ok"),
                               "failed_names": failures.group(1).split() if failures else []}
    return out


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, output = Path(sys.argv[1]), Path(sys.argv[2])
    result = {"tests": libtest(evidence)}
    plans = {p.name for v in ("baseline", "candidate") for p in (evidence / "captures" / v).glob("*")
             if (p / "report.json").exists()}
    result["cross_variant"] = {plan: compare_runs(evidence, ("baseline", plan), ("candidate", plan))
                               for plan in ("main-dry", "main", "shadow-dry", "shadow") if plan in plans}
    result["cross_process"] = {f"{v} {dry} vs {real}": compare_runs(evidence, (v, dry), (v, real))
                               for v in ("baseline", "candidate")
                               for dry, real in (("main-dry", "main"), ("shadow-dry", "shadow"))
                               if dry in plans and real in plans}
    result["shadow_controls"] = {
        v: [{"frame": f["step"], **{k: {x: y for x, y in value.items() if x != "rgba_sha256"}
                                    for k, value in f["controls"].items()}}
            for f in load(evidence, v, "shadow")[1]["frames"] if "controls" in f]
        for v in ("baseline", "candidate") if (evidence / "captures" / v / "shadow/report.json").exists()}
    if (evidence / "captures/candidate/source-free/report.json").exists():
        result["source_free"] = compare_runs(evidence, ("candidate", "main"), ("candidate", "source-free"))
    result["within_run"] = {v: within_run(evidence, v) for v in ("baseline", "candidate")
                            if (evidence / "captures" / v / "main/report.json").exists()}
    text = json.dumps(result, indent=1) + "\n"
    with open(output, "x") as out:
        out.write(text)
    summary = {"tests": {k: (v["exit"], v["summary"]) for k, v in result["tests"].items()},
               "cross_variant": {k: (v["frames_compared"], v["identical"]) for k, v in result["cross_variant"].items()},
               "cross_process": {k: (v["frames_compared"], v["identical"]) for k, v in result["cross_process"].items()},
               "source_free": (result["source_free"]["frames_compared"], result["source_free"]["identical"])
               if "source_free" in result else None}
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
