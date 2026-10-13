"""Evidence index for the 0070-ssao-filter-002 GPU comparison (filter-002-result.md).

Reads the released evidence directory and this directory's review aids; every
hash is computed here. Writes index.json create-new. No GPU, no Cargo.

  /opt/homebrew/bin/python3 -I -B make_index.py
"""

import hashlib
import json
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-002")
REVIEWS = Path("/Users/andreyyurlov/.codex/incant-reviews")
ORDER = ["control-filter_device", "control-ao_device", "control-ao_device_cooked", "case-ssao_core",
         "case-ssao_materials", "case-ssao_geometry", "case-lifecycle_drs", "case-ssao_quality", "appearance"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def ref(path):
    return {"path": str(path), "sha256": sha(path)}


def load(path):
    return json.loads(Path(path).read_text())


def children():
    by = {}
    for line in (E / "ledger.jsonl").read_text().splitlines():
        if line.strip():
            record = json.loads(line)
            by.setdefault(record["label"], {})[record["type"]] = record
    out = []
    for label in ORDER:
        reserve, done = by[label]["reserve"], by[label]["complete"]
        outcome, post = done["outcome"], done.get("postprocess") or {}
        out.append({"label": label, "kind": reserve["kind"], "pid": outcome["pid"], "exit": outcome["exit"],
                    "signal": outcome["signal"], "timed_out": outcome["timed_out"], "reaped": outcome["reaped"],
                    "group_absent": outcome["group_absent"], "survivors": outcome["survivors"],
                    "started": outcome["started"], "ended": outcome["ended"],
                    "seconds": round((datetime.fromisoformat(outcome["ended"])
                                      - datetime.fromisoformat(outcome["started"])).total_seconds(), 2),
                    "new_frames": reserve.get("new_frames"), "pngs": reserve.get("pngs"),
                    "binary_sha256": (reserve.get("binary") or {}).get("sha256"),
                    "binary_unchanged": done.get("binary_unchanged"), "success": done["success"],
                    "failure_reasons": done["failure_reasons"], "stdout_sha256": done["stdout_sha256"],
                    "retained_index": post.get("index"), "verdict_failures": post.get("verdict_failures")})
    return out


def main():
    target = HERE / "index.json"
    if target.exists():
        raise SystemExit(f"{target} exists; the index is never replaced")
    grant, release, analysis = load(E / "gpu-grant.json"), load(E / "released.json"), load(E / "filter-analysis.json")
    appearance = load(E / "outputs" / "appearance" / "index.json")
    review = HERE / "review"
    rows = {k: {r: {"p2p_255": v["p2p_255"], "bound_255": round(0.25 * v["shipped_255"], 2), "ratio": v["ratio"],
                    "pass": v["pass"]} for r, v in analysis["diagnostics"][k]["rows"].items()}
            for k in ("n8_rows_persp", "n8_rows_ortho")}
    index = {
        "format": "incant-0070-ssao-filter-002-index-v1",
        "status": "9 of 9 GPU children succeeded; released; numeric 43/43; revised quality pass with all 14 N8 rows "
                  "within 0.25; authored N6 failed (retained); appearance reviewed",
        "acceptance": "none granted here: no phase, cooked, default-on, device-class or performance approval",
        "source": {"commit": grant["source_commit"], "engine_commit": grant["engine_commit"],
                   "root_source_freeze": ref(E / "root-source-freeze.json")},
        "grant": {**ref(E / "gpu-grant.json"), **{k: grant[k] for k in ("at", "holder", "expected_backend",
                                                                         "plan_sha256", "helper_digest",
                                                                         "build_record_sha256",
                                                                         "build_review_sha256", "source_review")}},
        "reviews": {"build": ref(REVIEWS / "ssao-c2-filter002-build-root-20261013.json"),
                    "source": ref(REVIEWS / "ssao-c2-oracle-preparation-source-independent-20261013.json"),
                    "n6t_metric": ref(REVIEWS / "ssao-n6t-metric-root-20261013.json")},
        "prior_gpu_release": ref(grant["prior_shadow_release_path"]),
        "build": {"record": ref(E / "build" / "build.json"), "release": ref(E / "build-released.json"),
                  "refusals_retained": ref(E / "refusals.jsonl"),
                  "root_config_amendment": ref(E / "root-config-amendment.json")},
        "frozen_binaries": {k: v["sha256"] for k, v in load(E / "build" / "build.json")["frozen"].items()},
        "backend": appearance["summary"]["adapter"],
        "gpu_children": children(),
        "budgets": {"gpu_children": [release["children"], 9], "new_submissions": [release["new_frames_reserved"], 100, 128],
                    "pngs": [release["pngs_reserved"], 16], "pairs": [len(appearance["summary"]["pairs"]), 11],
                    "retries": release["retries"], "form": "[used, planned(, cap)]"},
        "release": {**ref(E / "released.json"), **{k: release[k] for k in ("released_at", "owned_processes_alive",
                                                                            "frozen_processes", "latched_failures",
                                                                            "ledger_sha256")}},
        "ledger": {**ref(E / "ledger.jsonl"), "equals_release": sha(E / "ledger.jsonl") == release["ledger_sha256"]},
        "outputs": {d.name: sha(d / "index.json") for d in sorted((E / "outputs").iterdir())},
        "analysis": {**ref(E / "filter-analysis.json"), "binding_plan_matches_build": analysis["binding"]["plan_matches_build"],
                     "numeric_gates": [sum(g["pass"] for g in analysis["numeric_gates"].values()),
                                       len(analysis["numeric_gates"])],
                     "quality_gates": {k: {kk: v.get(kk) for kk in ("pass", "step_residual_255", "count", "bound",
                                                                     "ratio", "p2p_255", "row")}
                                       for k, v in analysis["quality_gates"].items()},
                     "quality_pass": analysis["quality_pass"],
                     "authored_n6": {k: analysis["authored_n6"][k] for k in ("max_step_255", "at", "bound", "verdict")},
                     "authored_quality_pass": analysis["authored_quality_pass"],
                     "n6t_ortho_diagnostic": {k: analysis["diagnostics"]["n6t_ortho"][k]
                                              for k in ("step_residual_255", "at", "pass")},
                     "n8_rows": rows, "pass": analysis["pass"]},
        "appearance": {"pngs": {f["label"]: f["png_sha256"] for f in appearance["frames"]},
                       "pairs": appearance["summary"]["pairs"]},
        "review_aids": {"manifest": ref(review / "manifest.json"), "stripes_manifest": ref(review / "manifest-stripes.json"),
                        "pngs": {p.name: sha(p) for p in sorted(review.glob("*.png"))},
                        "ssao_brighter_than_off": load(review / "manifest.json")["ssao_brighter_than_off"]},
        "tool": {"make_index.py": sha(Path(__file__))},
    }
    with open(target, "x") as out:
        json.dump(index, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({"index_sha256": sha(target), "budgets": index["budgets"]}, indent=1))


if __name__ == "__main__":
    main()
