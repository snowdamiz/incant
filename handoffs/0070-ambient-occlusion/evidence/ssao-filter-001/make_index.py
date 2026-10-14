"""Evidence index for the 0070-ssao-filter-001 GPU comparison (filter-result.md).

Reads the released evidence directory and this directory's review and
next-step files; every hash is computed here, none is copied by hand. Writes
index.json create-new. No GPU, no Cargo, stdlib only.

  /opt/homebrew/bin/python3 -I -B make_index.py
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKET_DIR = HERE.parents[1]
E = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001")
PLAN = PACKET_DIR / "ssao-filter-001" / "plan.json"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def ref(path):
    return {"path": str(path), "sha256": sha(path)}


def load(path):
    return json.loads(Path(path).read_text())


def receipts(label):
    out = {}
    for line in (E / "logs" / f"{label}.stdout").read_text().splitlines():
        if line.startswith("EFFECTS-RECEIPT "):
            body = json.loads(line[len("EFFECTS-RECEIPT "):])
            out[body["kind"]] = body["body"]
    return out


def children(ledger):
    by_label = {}
    for record in ledger:
        if record["label"].startswith("build-"):
            continue
        by_label.setdefault(record["label"], {})[record["type"]] = record
    out = []
    for label, parts in by_label.items():
        reserve, spawned, done = parts["reserve"], parts["spawned"], parts["complete"]
        outcome, post = done["outcome"], done.get("postprocess") or {}
        out.append({
            "label": label,
            "kind": reserve["kind"],
            "control": reserve.get("control"),
            "target": reserve.get("target"),
            "argv": reserve["argv"],
            "binary_sha256": (reserve.get("binary") or {}).get("sha256"),
            "binary_unchanged": done.get("binary_unchanged"),
            "env_keys": reserve["env"]["keys"],
            "timeout_s": reserve["timeout_s"],
            "new_frames_reserved": reserve.get("new_frames"),
            "pngs_reserved": reserve.get("pngs"),
            "pid": spawned["pid"],
            "kernel_start": spawned["identity"]["start"],
            "started": outcome["started"],
            "ended": outcome["ended"],
            "exit": outcome["exit"],
            "signal": outcome["signal"],
            "timed_out": outcome["timed_out"],
            "reaped": outcome["reaped"],
            "group_absent": outcome["group_absent"],
            "survivors": outcome["survivors"],
            "success": done["success"],
            "failure_reasons": done["failure_reasons"],
            "stdout_sha256": done["stdout_sha256"],
            "stderr_sha256": done["stderr_sha256"],
            "verdict_failures": post.get("verdict_failures"),
            "retained_index": post.get("index"),
        })
    return out


def outputs(case):
    directory = E / "outputs" / f"case-{case}"
    index = load(directory / "index.json")
    rgba = {f["label"]: f["rgba_sha256"] for f in index["frames"] if "rgba_sha256" in f}
    planes = {f["label"]: f["r8_sha256"] for f in index["frames"] if "r8_sha256" in f}
    files_match = all(sha(directory / f["file"]) == f.get("rgba_sha256", f.get("r8_sha256"))
                      for f in index["frames"])
    return {"index_sha256": sha(directory / "index.json"), "retained_rgba_frames": len(rgba),
            "stage_planes": len(planes), "files_match_index": files_match, "rgba": rgba, "r8": planes}


def main():
    target = HERE / "index.json"
    if target.exists():
        raise SystemExit(f"{target} exists; the index is never replaced")
    grant = load(E / "gpu-grant.json")
    plan = load(PLAN)
    release = load(E / "released.json")
    build_release = load(E / "build-released.json")
    build_review = load(E / "build-review-root.json")
    analysis = load(E / "filter-analysis.json")
    diagnostic = load(E / "sphere-count-diagnostic-root.json")
    ledger = [json.loads(line) for line in (E / "ledger.jsonl").read_text().splitlines() if line.strip()]
    gpu = children(ledger)
    quality = receipts("case-ssao_quality")
    adapters = {json.dumps(d["adapter"], sort_keys=True)
                for c in gpu if c["kind"] == "numeric" for d in receipts(c["label"])["devices"]}
    if len(adapters) != 1:
        raise SystemExit(f"cases ran on {len(adapters)} adapters")
    frozen = analysis["binding"]["frozen"]
    binaries = {name: {"sha256": sha(E / "bin" / name), "bytes": (E / "bin" / name).stat().st_size,
                       "equals_analysis_binding": sha(E / "bin" / name) == frozen[name]}
                for name in sorted(frozen)}
    planned = [f"control-{c['name']}" for c in plan["controls"]] + \
              [f"case-{c['name']}" for c in plan["numeric_cases"]] + ["appearance"]
    executed = [c["label"] for c in gpu]
    review = HERE / "review"
    next_step = HERE / "next-step"
    jitter = load(next_step / "per-ray-jitter-001.json")
    steps8 = load(next_step / "steps8-full-stratum-001.json")
    index = {
        "format": "incant-0070-ssao-filter-001-index-v1",
        "packet": grant["packet"],
        "namespace": grant["namespace"],
        "status": ("halted: case-ssao_quality failed its own numerical-contract guard (sphere coverage), "
                   "so device work stopped before appearance; GPU released; analyzer run after release"),
        "acceptance": ("none granted here: no phase gate, cooked, default-on or phone approval; the filter is "
                       "not accepted while N6 authored and N8 perspective fail and appearance is unrendered"),
        "source": {"head": grant["source_commit"], "engine_commit": grant["engine_commit"],
                   "preflight": ref(E / "preflight-source.json")},
        "evidence_dir": str(E),
        "grant": {**ref(E / "gpu-grant.json"),
                  **{k: grant[k] for k in ("at", "holder", "expected_backend", "plan_sha256", "helper_digest",
                                           "build_record_sha256", "build_review_sha256", "source_review")}},
        "plan": {**ref(PLAN), "equals_grant": sha(PLAN) == grant["plan_sha256"]},
        "build": {
            "grants": [ref(E / "build-grant.json"), ref(E / "build-grant-002.json")],
            "refusal_before_any_child": ref(E / "refusals.jsonl"),
            "record": ref(E / "build" / "build.json"),
            "release": {**ref(E / "build-released.json"), "released_at": build_release["released_at"],
                        "children": build_release["build_children"]},
            "root_review": {**ref(E / "build-review-root.json"), "findings": build_review["findings"],
                            "scope": build_review["scope"]},
        },
        "frozen_binaries": binaries,
        "feature_binding": analysis["binding"]["feature_binding"],
        "backend": json.loads(adapters.pop()),
        "sequence": {"planned": planned, "executed": executed,
                     "not_run": [label for label in planned if label not in executed],
                     "why_not_run": "every child failure halts device work (brief); no retry"},
        "budgets": {"gpu_children": [len(gpu), plan["budgets"]["gpu_children_max"]],
                    "new_submissions": [release["new_frames_reserved"],
                                        plan["budgets"]["new_frames_planned"]["total"],
                                        plan["budgets"]["new_frames_max"]],
                    "pngs": [release["pngs_reserved"], plan["appearance"]["pngs"]],
                    "pairs": [0, plan["appearance"]["pairs"]],
                    "retries": release["retries"],
                    "form": "[used, planned(, cap)]",
                    "lifecycle_drs": "18 submissions counted; l_invalid_viewport retains no frame, so 17 are indexed"},
        "gpu_children": gpu,
        "outputs": {c["name"]: outputs(c["name"]) for c in plan["numeric_cases"]},
        "run_contract_failure": {
            "child": "case-ssao_quality",
            "checks": [c for c in quality["verdict"]["checks"] if not c["ok"]],
            "guard": "ssao-filter-001/effects/src/numeric.rs sphere_texels (texel centre inside RECT)",
            "cause": ("the 601 viewport texels only partly covered by the fractional viewport (13.25, 17.5, "
                      "280.5, 160.5), columns x 13 and x 293 and row y 17, blend the canvas into the scene "
                      "background, so they differ from the background probe at (16, 20) and were counted"),
            "rim_colours": diagnostic["frames"][0]["edge_colors"],
            "interior_counts": {f["label"]: {"all": f["all_nonbackground"],
                                             "rim": f["within_one_pixel_viewport_edge"],
                                             "interior": f["interior_nonbackground"]}
                                for f in diagnostic["frames"]},
            "analyzer_interior_domain": {
                k: {"sphere_texels": analysis["numeric_gates"][k]["sphere_texels"],
                    "expected_area": analysis["numeric_gates"][k]["expected_area"],
                    "pass": analysis["numeric_gates"][k]["pass"]}
                for k in ("ssao_quality_p_sphere_identity", "ssao_quality_o_sphere_identity")},
            "root_diagnostic": ref(E / "sphere-count-diagnostic-root.json"),
        },
        "release": {**ref(E / "released.json"),
                    **{k: release[k] for k in ("released_at", "children", "new_frames_reserved", "pngs_reserved",
                                               "retries", "owned_processes_alive", "frozen_processes",
                                               "latched_failures", "ledger_sha256")}},
        "ledger": {**ref(E / "ledger.jsonl"), "equals_release": sha(E / "ledger.jsonl") == release["ledger_sha256"]},
        "analysis": {**ref(E / "filter-analysis.json"),
                     "command": "env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B "
                                "handoffs/0070-ambient-occlusion/ssao-repair/filter/analyze_filter.py <evidence>",
                     "observed_at_run": {"exit_status": 1, "wall_s": 103,
                                         "note": "printed by the shell after the analyzer; the file records pass false"},
                     "numeric_pass": analysis["numeric_pass"],
                     "numeric_gates": [sum(1 for g in analysis["numeric_gates"].values() if g["pass"]),
                                       len(analysis["numeric_gates"])],
                     "quality_pass": analysis["quality_pass"],
                     "failed_quality_gates": analysis["failed_quality_gates"],
                     "quality_gates": analysis["quality_gates"],
                     "diagnostics": analysis["diagnostics"],
                     "appearance": analysis["appearance"]},
        "review_aids": {"manifest": ref(review / "manifest.json"), "tool": ref(review / "make_review.py"),
                        "pngs": {p.name: sha(p) for p in sorted(review.glob("*.png"))},
                        "note": "derived after release from retained RGBA; not renders, not the 16 appearance PNGs"},
        "next_step_checks": [
            {"tool": ref(next_step / "per_ray_jitter.py"), "result": ref(next_step / "per-ray-jitter-001.json"),
             "candidate": jitter["candidate"],
             "n8_persp_rows": {r: v["candidate_ratio"] for r, v in jitter["n8_persp"]["rows"].items()},
             "n8_ortho": jitter["n8_ortho"]["rows"], "n6_authored_255": jitter["n6_authored_255"],
             "n7": {k: v["count"] for k, v in jitter["n7"].items()},
             "verdict": "fails N8 perspective: 0.439 at the ROI row 124, worse than the design's 0.376"},
            {"tool": ref(next_step / "steps8_full_stratum.py"),
             "result": ref(next_step / "steps8-full-stratum-001.json"),
             "candidate": steps8["candidate"],
             "n8_persp_rows": {r: v["steps8_full_stratum"] for r, v in steps8["n8_persp"]["rows"].items()},
             "n8_ortho": steps8["n8_ortho"]["rows"], "n6t_persp_255": steps8["n6t_persp_255"],
             "n6_authored_persp_255": steps8["n6_authored_persp_255"],
             "n7": {k: v["count"] for k, v in steps8["n7_candidate"].items()},
             "self_check_rows_match_addendum": steps8["self_check_rows_match_addendum"],
             "verdict": "fails N8 perspective: 0.314 at the ROI row 124, at about twice the AO pass cost"},
        ],
        "tool": {"make_index.py": sha(Path(__file__))},
    }
    with open(target, "x") as out:
        json.dump(index, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({"index": str(target), "sha256": sha(target), "children": executed,
                      "not_run": index["sequence"]["not_run"], "budgets": index["budgets"]}, indent=1))


if __name__ == "__main__":
    main()
