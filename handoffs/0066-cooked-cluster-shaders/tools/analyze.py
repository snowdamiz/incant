#!/usr/bin/env python3
"""CPU analysis of handoff 0066 review captures and GPU test logs. Standard library only.

Adapted from handoff 0064's committed tools/analyze.py (read-only reuse).

  python3 -I -B analyze.py <evidence-dir> <new-output.json>

Reads captures/<variant>/<plan>/report.json and PNGs, gpu-ledger.jsonl,
gpu-refusals.jsonl and the libtest logs; never starts a renderer. Every frame
comparison is on unmasked whole frames: PNG bytes, decoded RGBA SHA-256
(checked against the helper's own readback hash) and per-row hashes.

Numerical cluster membership is recorded separately from frames: the existing
crate-internal readback tests (tests/lighting_gpu/mod.rs) assert every count
and mask word against closed-form oracles, which are reconstructed here with
their coverage so a pass in both variants states exactly which arrays matched.
"""

import hashlib
import json
import re
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
READBACK_SOURCE = ROOT / "crates/incant_render/tests/lighting_gpu/mod.rs"
SWEEP = [31, 32, 33, 63, 64, 65, 127, 128, 129, 4095, 4096, 65, 1]
VARIANTS = ("baseline", "candidate")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def key_of(frame):
    vp = "+vp" if frame.get("viewport") else ""
    return f"{frame['step']}@{frame['tick']}:{frame['width']}x{frame['height']}{vp}"


def load(evidence, variant, plan):
    directory = evidence / "captures" / variant / plan
    report = json.loads((directory / "report.json").read_text())
    seen = {}
    for frame in report["frames"]:
        base = key_of(frame)
        seen[base] = seen.get(base, -1) + 1
        frame["key"] = f"{base}#{seen[base]}"  # occurrence within the run
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
    """Frame-by-frame comparison of two runs (of the same plan, or a frame subset by key)."""
    (dir_a, rep_a), (dir_b, rep_b) = load(evidence, *a), load(evidence, *b)
    frames_b = {f["key"]: f for f in rep_b["frames"]}
    rows = []
    for fa in rep_a["frames"]:
        fb = frames_b.get(fa["key"])
        if fb is None:
            continue
        entry = {"frame": fa["key"], "rgba_sha256_a": fa["rgba_sha256"], "rgba_sha256_b": fb["rgba_sha256"],
                 "identical": fa["rgba_sha256"] == fb["rgba_sha256"],
                 "differing_rows": sum(x != y for x, y in zip(fa["row_hashes"], fb["row_hashes"])),
                 "lights_a": fa["lights"], "lights_equal": fa["lights"] == fb["lights"],
                 "update_equal": fa["update"] == fb["update"]}
        if "png" in fa and "png" in fb:
            pa, pb = dir_a / fa["png"]["name"], dir_b / fb["png"]["name"]
            wa, ha, rows_a, da = decode(pa)
            wb, hb, rows_b, db = decode(pb)
            entry["png"] = {"a": {"name": pa.name, "sha256": sha(pa), "decoded_rgba_sha256": da,
                                  "matches_readback": da == fa["rgba_sha256"],
                                  "chunks": pngio.chunk_types(pa.read_bytes())},
                            "b": {"name": pb.name, "sha256": sha(pb), "decoded_rgba_sha256": db,
                                  "matches_readback": db == fb["rgba_sha256"]},
                            "bytes_identical": sha(pa) == sha(pb)}
            entry["pixels"] = pixel_diff(rows_a, rows_b, wa) if (wa, ha) == (wb, hb) else {"error": "size"}
        rows.append(entry)
    return {"a": "/".join(a), "b": "/".join(b), "frames_compared": len(rows),
            "identical": sum(r["identical"] for r in rows),
            "lights_equal": sum(r["lights_equal"] for r in rows), "frames": rows}


def by_key(report, key):
    found = [f for f in report["frames"] if f["key"] == key]
    return found[0] if found else None


def rows_differ(x, y):
    return sum(p != q for p, q in zip(x["row_hashes"], y["row_hashes"]))


def within_run(evidence, variant, plan="main"):
    directory, report = load(evidence, variant, plan)
    checks = {}
    persp = [f for f in report["frames"] if f["step"] == "C-persp"]
    checks["C-persp sequence"] = [{"frame": f["key"], "update": f["update"], "prepared": f["retained_scenes_prepared"],
                                   "entities": f["entities"], "lights": f["lights"], "rgba_sha256": f["rgba_sha256"]}
                                  for f in persp]
    k = lambda key: by_key(report, key)  # noqa: E731
    pairs = {
        "t0 -> t15 (retained motion, same 15x9 grid)": ("C-persp@0:960x540#0", "C-persp@15:960x540#0"),
        "t15 -> t30 (retained motion, same 15x9 grid)": ("C-persp@15:960x540#0", "C-persp@30:960x540#0"),
        "t30 -> t31 (4096 -> 33 local lights)": ("C-persp@30:960x540#1", "C-persp@31:960x540#0"),
        "t31 -> t32 (33 -> 32)": ("C-persp@31:960x540#0", "C-persp@32:960x540#0"),
        "t32 -> t33 (32 -> 31)": ("C-persp@32:960x540#0", "C-persp@33:960x540#0"),
        "t33 -> t34 (31 -> 1)": ("C-persp@33:960x540#0", "C-persp@34:960x540#0"),
        "t34 -> t35 (1 -> 0, directional only)": ("C-persp@34:960x540#0", "C-persp@35:960x540#0"),
        "t35 -> t36 (no light, nothing moves)": ("C-persp@35:960x540#0", "C-persp@36:960x540#0"),
    }
    checks["consecutive"] = {name: {"from": a, "to": b, "identical": k(a)["rgba_sha256"] == k(b)["rgba_sha256"],
                                    "differing_rows": rows_differ(k(a), k(b))}
                             for name, (a, b) in pairs.items() if k(a) and k(b)}
    same = {
        "t30 960x540 after resizes and an offset viewport repeats the first t30 frame":
            ("C-persp@30:960x540#0", "C-persp@30:960x540#1"),
        "C-persp t0 rendered again after ortho/D churn": ("C-persp@0:960x540#0", "C-persp-again@0:960x540#0"),
    }
    checks["repeats"] = {name: {"a": a, "b": b, "identical": k(a)["rgba_sha256"] == k(b)["rgba_sha256"],
                                "differing_rows": rows_differ(k(a), k(b))}
                         for name, (a, b) in same.items() if k(a) and k(b)}
    vp, small = k("C-persp@30:960x540+vp#0"), k("C-persp@30:640x360#0")
    if vp and small and "png" in vp:
        width, height, rows, _ = decode(directory / vp["png"]["name"])
        x0, y0, w, h = (int(v) for v in vp["viewport"])
        inner = [hashlib.sha256(rows[y0 + y][4 * x0:4 * (x0 + w)]).hexdigest()[:16] for y in range(h)]
        canvas = bytes([20, 21, 25, 255])
        outside = sum(1 for y in range(height) for x in range(width)
                      if not (x0 <= x < x0 + w and y0 <= y < y0 + h) and rows[y][4 * x:4 * x + 4] != canvas)
        checks["viewport"] = {"frame": vp["key"], "rect": vp["viewport"],
                              "inner_rows_equal_640x360_frame": sum(a == b for a, b in zip(inner, small["row_hashes"])),
                              "inner_rows": h, "outside_pixels_not_canvas_20_21_25": outside}
    controls = []
    for f in report["frames"]:
        if "controls" in f:
            controls.append({"frame": f["key"], **{name: {x: y for x, y in value.items() if x != "rgba_sha256"}
                                                   for name, value in f["controls"].items()}})
    checks["controls"] = controls
    checks["controls_summary"] = {
        name: {"frames": sum(1 for x in controls if name in x),
               "exact": sum(1 for x in controls if x.get(name, {}).get("differing_pixels") == 0),
               "worst_delta": max((x[name]["max_channel_delta"] for x in controls if name in x), default=None),
               "worst_pixels": max((x[name]["differing_pixels"] for x in controls if name in x), default=None)}
        for name in ("retained_repeat", "nonretained_clustered", "nonretained_all_lights")}
    checks["adapter"] = report["adapter"]
    checks["pngs_written"] = report["pngs_written"]
    checks["frames"] = len(report["frames"])
    checks["seconds"] = report["seconds"]
    checks["black_frames"] = [f["key"] for f in report["frames"] if f["black_pixels"] == f["width"] * f["height"]]
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
        tests = re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", text, re.M)
        failures = re.search(r"^failures:\n\n?((?:    \S+\n)+)", text, re.M)
        failed = failures.group(1).split() if failures else []
        out[entry["label"]] = {
            "exit": entry["exit"], "timed_out": entry["timed_out"], "pid": entry["pid"], "argv": entry["argv"][1:],
            "started": entry["started"], "ended": entry["ended"], "summary": summary,
            "completed": sorted(n for n, r in tests if r == "ok" and n not in failed),
            "failed": sorted(set(failed) | {n for n, r in tests if r == "FAILED"}),
            "skipped": sorted(n for n, r in tests if r == "ignored")}
    return out


def listings(evidence):
    out = {}
    for path in sorted((evidence / "gpu-logs").glob("*-list.stdout.txt")):
        names = re.findall(r"^(\S+): test$", path.read_text(), re.M)
        out[path.name.removesuffix(".stdout.txt")] = sorted(names)
    return out


def source_union():
    """#[ignore] sites in crates/, grouped by the test binary that compiles them."""
    sites = []
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        for number, line in enumerate(path.read_text().splitlines(), 1):
            if line.strip().startswith("#[ignore"):
                sites.append(f"{path.relative_to(ROOT)}:{number}")
    groups = {"incant_render lib (src/*_tests.rs and #[path] tests/{diagnostic_output,hdr_output,frame_graph,lighting_gpu})": 0,
              "incant_render model_gpu": 0, "incant_render_gpu_host game": 0, "incant_render_gpu_host ui": 0,
              "incant_headless model_gpu (legacy document Play recorder; outside the four suites)": 0, "other": 0}
    lib_dirs = ("diagnostic_output", "hdr_output", "frame_graph", "lighting_gpu")
    for site in sites:
        p = site.split(":")[0]
        if p.startswith("crates/incant_render/src/") or any(f"crates/incant_render/tests/{d}/" in p for d in lib_dirs):
            groups[next(iter(groups))] += 1
        elif p.startswith("crates/incant_render/tests/"):
            groups["incant_render model_gpu"] += 1
        elif p == "crates/incant_render_gpu_host/tests/game.rs":
            groups["incant_render_gpu_host game"] += 1
        elif p == "crates/incant_render_gpu_host/tests/ui.rs":
            groups["incant_render_gpu_host ui"] += 1
        elif p.startswith("crates/incant_headless/"):
            groups["incant_headless model_gpu (legacy document Play recorder; outside the four suites)"] += 1
        else:
            groups["other"] += 1
    return {"sites": len(sites), "groups": groups, "all_sites": sites}


def words_le(values):
    return b"".join(struct.pack("<I", v) for v in values)


def readback_oracle(tests):
    """Exact arrays the existing readback tests assert (5x3x24 = 360 clusters at 320x192)."""
    clusters = 5 * 3 * 24
    sweep = []
    for count in SWEEP:
        words = (count + 31) // 32
        mask = [0xFFFFFFFF >> (32 - min(32, count - 32 * w)) for w in range(words)]
        counts, masks = [count] * clusters, mask * clusters
        sweep.append({"local_lights": count, "mask_words": words, "counts_sha256": hashlib.sha256(words_le(counts)).hexdigest(),
                      "masks_sha256": hashlib.sha256(words_le(masks)).hexdigest(), "counts_len": len(counts),
                      "masks_len": len(masks), "asserted": "every count and every mask word (complete)"})
    depth_counts = [0] * clusters
    for z in range(24):
        depth_counts[(z * 3 + 1) * 5 + 2] = 1
    out = {
        "source": str(READBACK_SOURCE.relative_to(ROOT)), "source_sha256": sha(READBACK_SOURCE),
        "readback_fn": "grid_readback: LightingSystem::prepare + ClusterPass::encode, then copies the cached "
                       "GridBuffers counts/masks (COPY_SRC) to a MAP_READ buffer",
        "viewport": [0, 0, 320, 192], "grid": [5, 3, 24], "camera": "CameraView::preview() (perspective)",
        "masks_represent_every_light_and_clear_tail_bits_across_word_count_changes": {
            "sequence": SWEEP, "geometry": "N point lights at the origin, range 10000: every cluster holds every light",
            "configurations": sweep},
        "every_depth_slice_selects_its_light_and_reused_clusters_clear_old_membership": [
            {"lights": "1 directional + 24 local (one tiny light per depth slice on the camera axis)",
             "counts_sha256": hashlib.sha256(words_le(depth_counts)).hexdigest(),
             "asserted": "counts complete (24 named clusters = 1, sum = 24); masks at the 24 named clusters "
                         "= 1 << z (directional prefix excluded from local bits); other mask words not asserted"},
            {"lights": "1 local light at x = 1e6 (intersects nothing), after the 24-light dispatch",
             "counts_sha256": hashlib.sha256(words_le([0] * clusters)).hexdigest(),
             "masks_sha256": hashlib.sha256(words_le([0] * clusters)).hexdigest(),
             "asserted": "every count and mask word zero (complete; stale membership cleared)"}],
        "directional_only": "LightingSystem::prepare returns compute: None when local == 0, so no cluster "
                            "dispatch exists to read back; the frame review covers that path"}
    names = ["lighting::gpu_tests::masks_represent_every_light_and_clear_tail_bits_across_word_count_changes",
             "lighting::gpu_tests::every_depth_slice_selects_its_light_and_reused_clusters_clear_old_membership",
             "lighting::gpu_tests::clustered_viewport_offsets_and_queued_resize_commands_keep_their_own_light_data"]
    out["results"] = {label: {n: ("completed" if n in r["completed"] else "failed" if n in r["failed"] else "absent")
                              for n in names}
                      for label, r in tests.items() if "-lib-" in label}
    return out


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, output = Path(sys.argv[1]), Path(sys.argv[2])
    tests = libtest(evidence)
    result = {"tests": tests, "listings": listings(evidence), "source_union": source_union()}
    refusals = evidence / "gpu-refusals.jsonl"
    result["refusals"] = [json.loads(x) for x in refusals.read_text().splitlines()] if refusals.exists() else []
    result["readback_oracle"] = readback_oracle(tests)
    plans = {p.name for v in VARIANTS for p in (evidence / "captures" / v).glob("*") if (p / "report.json").exists()}
    result["cross_variant"] = {plan: compare_runs(evidence, ("baseline", plan), ("candidate", plan))
                               for plan in ("main-dry", "main") if plan in plans}
    result["cross_process"] = {f"{v} main-dry vs main": compare_runs(evidence, (v, "main-dry"), (v, "main"))
                               for v in VARIANTS
                               if (evidence / "captures" / v / "main-dry/report.json").exists()
                               and (evidence / "captures" / v / "main/report.json").exists()}
    if (evidence / "captures/candidate/source-free/report.json").exists():
        result["source_free"] = compare_runs(evidence, ("candidate", "main"), ("candidate", "source-free"))
    result["within_run"] = {v: within_run(evidence, v) for v in VARIANTS
                            if (evidence / "captures" / v / "main/report.json").exists()}
    text = json.dumps(result, indent=1) + "\n"
    with open(output, "x") as out:
        out.write(text)
    summary = {"tests": {k: (v["exit"], v["summary"], len(v["completed"]), len(v["failed"]), len(v["skipped"]))
                         for k, v in tests.items()},
               "listings": {k: len(v) for k, v in result["listings"].items()},
               "source_union": result["source_union"]["groups"], "refusals": len(result["refusals"]),
               "cross_variant": {k: (v["frames_compared"], v["identical"]) for k, v in result["cross_variant"].items()},
               "cross_process": {k: (v["frames_compared"], v["identical"]) for k, v in result["cross_process"].items()},
               "source_free": (result["source_free"]["frames_compared"], result["source_free"]["identical"])
               if "source_free" in result else None}
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
