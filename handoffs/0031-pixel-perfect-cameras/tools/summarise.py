#!/usr/bin/env python3
"""Compact before/after summary of the handoff 0031 corrected-renderer follow-up.

Reads the analysis and helper JSON written by analyse.py, analyse_follow.py and
analyse_neartie.py (and the runs they name) and writes one small JSON. Each run is
also reduced to a digest: sha256 over its sorted "capture/frame sha256" lines,
so another from-scratch run can be checked by exact hash without the raw frames.

Usage (repo root), every path under artifacts/0031-pixel-perfect-cameras:
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/summarise.py ARTIFACTS_DIR OUT_JSON
OUT_JSON must not exist.
"""

import hashlib
import json
import sys
from pathlib import Path


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def digest(helper):
    """Frame-set digest of one helper.json, for either helper layout."""
    lines = []
    if "screenshots" in helper:  # pixel_lookdev.py
        for name, entry in helper["screenshots"].items():
            lines.append(f"shots/{name}-r1.png {entry['r1']}")
        for name, entry in helper["plays"].items():
            lines += [f"live-{name}/{frame} {sha}" for frame, sha in entry["frames"].items()]
    else:  # pixel_follow.py / pixel_neartie.py
        for capture, entry in helper["plays"].items():
            lines += [f"{capture}/{frame} {sha}" for frame, sha in entry["frames"].items()]
    lines.sort()
    return {"frames": len(lines), "sha256": hashlib.sha256("\n".join(lines).encode()).hexdigest()}


def follow_view(analysis):
    configs = {}
    for key, entry in analysis["configs"].items():
        configs[key] = {
            "followers_exact": {k: f"{v['exact_frames']}/{entry['frames']}" for k, v in entry["followers"].items()},
            "follower_offsets": {k: v["offsets"] for k, v in entry["followers"].items()},
            "cells_identical": f"{entry['cells']['identical']}/{entry['cells']['count']}",
            "background_exact": f"{entry['background_shift']['exact_frames']}/{entry['background_shift']['frames']}",
            "background_max_mismatch": entry["background_shift"]["max_mismatch"],
        }
    return {"binary_sha256": analysis["binary_sha256"], "rule": analysis["rule"], "summary": analysis["summary"],
            "repeat": {k: analysis["repeat"][k] for k in ("compared", "equal")}, "configs": configs}


def neartie_view(analysis):
    configs = {}
    for key, entry in analysis["configs"].items():
        series = {}
        for name in ("right+", "right-", "up+"):
            rows = [r for r in entry["rows"] if r["series"] == name]
            bad = [r for r in rows if r["a"] != [0, 0] or r["b"] != [0, 0]]
            series[name] = {
                "displaced": f"{len(bad)}/{len(rows)}",
                "exact_tie_displaced": any(r["delta"] == 0 for r in bad),
                "displaced_deltas": sorted(r["delta"] for r in bad),
                "offsets": sorted({json.dumps([r["a"], r["b"]]) for r in bad}),
            }
        configs[key] = {"followers_correct": f"{entry['followers_correct']}/{entry['frames']}", "series": series}
    return {"binary_sha256": analysis["binary_sha256"], "repeat": {k: analysis["repeat"][k] for k in ("compared", "equal")},
            "configs": configs}


def main():
    base, out = Path(sys.argv[1]), Path(sys.argv[2])
    if out.exists():
        raise SystemExit("Output exists; choose a new file.")
    original = load(base / "analysis-corrected-r1.json")
    live = original["live"]
    summary = {
        "binaries": {
            "corrected": {"source": "72ede59a7d1a908439f18c38bf35f47b07b5ddd4",
                          "sha256": load(base / "engine-corrected-r1/helper.json")["binary_sha256_start"]},
            "foundation": {"source": "04a39acf7ff87aa24f4996fc9672fca6b782c00e",
                           "sha256": load(base / "follow-foundation-r1/helper.json")["binary_sha256_start"]},
        },
        "original_suite_corrected": {
            "rule": original["rule"],
            "base_replication_exact": all(v["mismatched_inside"] == 0 and v["mismatched_letterbox"] == 0
                                          for v in original["base_replication"].values()),
            "other_configs_exact": all(e["mismatched_inside"] == 0 and e["mismatched_letterbox"] == 0
                                       for v in original["other_configs"].values() for e in v.values()),
            "static_motion_as_expected": f"{sum(v['as_expected'] for v in original['motion_1x'].values())}/{len(original['motion_1x'])}",
            "static_motion": {k: {"measured": [v["dx"], v["dy"]], "expected": v["expected"]}
                              for k, v in original["motion_1x"].items()},
            "roll": {k: original["roll"][k] for k in ("roll90_equals_rig90_base", "rig_expected")},
            "live": {k: live[k]["summary"] for k in ("1x", "diag-1x", "diag2-1x")},
            "orthographic_segment_mismatch": [[e["tick"], e["mismatched"]] for e in live["1x"]["frames"]
                                              if e["projection"] == "orthographic"],
            "live_3x_odd_pixel_frames_replicating": f"{sum(1 for e in live['frames_3x_odd'] if e.get('replicates_1x_frame'))}/{len(live['frames_3x_odd'])}",
            "repeat": {k: original["from_scratch_comparison"][k] for k in ("compared", "equal")},
            "versus_foundation": {k: original["baseline_comparison"][k] for k in
                                  ("compared", "identical", "changed", "unexpected_changes", "predicted_but_identical")},
        },
        "follow": {b: follow_view(load(base / f"follow-analysis-{b}-r1.json")) for b in ("foundation", "corrected")},
        "neartie": {b: neartie_view(load(base / f"neartie-analysis-{b}-r1.json")) for b in ("foundation", "corrected")},
        "run_digests": {name: digest(load(base / name / "helper.json")) for name in (
            "engine-r3", "engine-r4", "engine-corrected-r1", "engine-corrected-r2",
            "follow-foundation-r1", "follow-foundation-r2", "follow-corrected-r1", "follow-corrected-r2",
            "neartie-foundation-r1", "neartie-foundation-r2", "neartie-corrected-r1", "neartie-corrected-r2")},
    }
    out.write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps(summary["run_digests"], indent=1))


if __name__ == "__main__":
    main()
