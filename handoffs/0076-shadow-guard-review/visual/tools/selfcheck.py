#!/usr/bin/env python3
"""Preparation self-check for handoff 0076's shadow receiver guard comparison.
Standard library only; no grant needed. Never compiles, starts a renderer,
acquires a target or touches the stage evidence directory or the global GPU lock.
Adapted from 0072's selfcheck.py (48a688c5).

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every stage Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent and within the brief's budgets: 26 world
   submissions and 2 PNGs per role, read from the fixture itself; 52 planned
   within 60; 4 of 4 PNGs at 960x540; one viewport attempt per role.
3. Sources (git objects only): both commits exist, the candidate's parent is the
   baseline, and the plan's delta is exactly `git diff` between them, with no
   shader, cooker or pipeline file and only the four production files reaching
   the review binary.
4. The fixture is 0072's committed fixture (d4fc4002...) plus exactly one inserted
   line, inside ui_input, which the viewport case never calls; main224 and the
   candidate require that UiWidget field and 0072's source lacked it. The
   fixture agrees with the plan: test names, the viewport case's Report::open
   limits, PNG names, frame size, output variable, review config and contract
   checks; 0072's plan describes the viewport case identically.
5. The grille-shadow region reproduces 0072's cascade_texels.txt card-shadow
   lines exactly.
6. Calibration: the plan's control values recompute from the retained 0072
   evidence at its commit, the plan's limits equal the rule's, every consistent
   control meets every agreement limit and every mismatch control fails each one.
7. The planned image pairs build on CPU from retained 0072 PNGs (a temporary
   directory, deleted afterwards).
8. With both grants absent, every guarded command refuses with its grant
   message, creates no evidence directory, leaves `git status` unchanged and
   does not touch the global native-gpu.lock.
"""

import ast
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

FRAMEWORK = "48a688c58f15ce2951292471c5d37cd2cc1fdc9b"
FRAMEWORK_VISUAL = "handoffs/0072-motion-history-numeric/visual"
BUILT_0072 = "a143b5c0f537bd35111e40e99626f40318a385ac"  # the source the 0072 fixture last compiled against
HALOS = "pub halos: Option<Arc<[GlyphHandle]>>"
COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"],
            *[["build.py", step, role] for role in c.ROLES for step in ("checkout", "toolchain", "review", "lint")],
            ["gpu.py", "list", "baseline"], ["gpu.py", "review", "baseline", "viewport"],
            ["gpu.py", "list", "candidate"], ["gpu.py", "review", "candidate", "viewport"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "compare"], ["analyze.py", "pairs"], ["analyze.py", "publish"]]
ABSENT = ("build-ready.json is absent", "review-ready.json is absent")


def blob(spec):
    return subprocess.run(["git", "cat-file", "blob", spec], cwd=c.ROOT, capture_output=True, check=True).stdout


def case_body(source, name):
    """A top-level fn's text, from `fn name(` to its closing brace in column 0."""
    body = re.search(r"\nfn " + name + r"\((.*?)\n\}", source, re.S)
    return body[1] if body else ""


def case_checks(source, name):
    block = re.search(r"let checks = \[(.*?)\n    \];", case_body(source, name), re.S)
    return re.findall(r'\(\s*"([a-z0-9_]+)",', block[1]) if block else []


def plan_checks(p, fixture):
    spec = p["review"]
    case = spec["cases"]["viewport"]
    budgets, totals = p["budgets"], p["totals"]
    opened = re.search(r'Report::open\("viewport", (\d+), (\d+)\)', fixture)
    pngs = re.findall(r'report\.png\("([\w-]+\.png)"', case_body(fixture, "viewport"))
    charges = len(re.findall(r"shoot_viewport\(", case_body(fixture, "viewport")))
    return {
        "viewport_limits_from_fixture_26_0": bool(opened) and (int(opened[1]), int(opened[2])) == (26, 0)
        == (case["frames"], case["ui_submissions"]),
        "viewport_frames_counted_in_fixture": charges == 3 and "for k in 1..=24_u32" in case_body(fixture, "viewport"),
        "viewport_pngs_from_fixture": pngs == case["pngs"] and len(pngs) == 2,
        "per_role_totals": totals["frames_per_role"] == case["frames"] == 26
        and totals["frames"] == len(c.ROLES) * 26 == 52 and totals["ui_submissions"] == 0,
        "planned_52_within_60": totals["submissions"] == budgets["planned_submissions"] == 52
        and budgets["submissions_max"] == 60 and totals["reserve_submissions"] == 8,
        "pngs_4_of_4": totals["pngs"] == len(c.ROLES) * len(case["pngs"]) == budgets["pngs_max"] == 4
        and totals["reserve_pngs"] == 0,
        "reserve_admits_no_retry": totals["reserve_submissions"] < case["frames"],
        "size_960x540": spec["size"] == [960, 540] and budgets["frame_max_width"] == 960
        and budgets["frame_max_height"] == 540,
        "one_case_one_attempt_per_role": spec["order"] == ["viewport"] and sorted(spec["cases"]) == ["viewport"]
        and spec["max_runs_per_case"] == 1,
        "gpu_children_at_most_120_s": all(budgets[k] <= 120 for k in ("listing_timeout_s", "review_case_timeout_s")),
        "no_numeric_or_suite": not any(k in p for k in ("numeric", "suites"))
        and sorted(p["builds"]) == ["common_flags", "expected_features", "lint", "note", "review"],
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2"))
        and budgets["compile_jobs"] == 2,
        "default_features_only": p["builds"]["expected_features"] == ["default", "physics-deterministic"],
        "review_config_in_validated_ranges": (
            0 <= p["review_config"]["history_weight"] <= 0.99 and 0 <= p["review_config"]["relative_depth"] <= 0.1
            and 0 <= p["review_config"]["absolute_depth"] <= 1
            and 1e-6 <= p["review_config"]["reactive_threshold"] <= 65504
            and 0 <= p["review_config"]["reactive_strength"] <= 1),
        "expected_backend_metal": p["expected_backend"] == "Metal",
        "new_evidence_namespace": p["evidence_namespace"] == str(c.EVIDENCE)
        and c.EVIDENCE.name == "0076-shadow-guard-review-001" and c.GRANT.parent == c.EVIDENCE
        and c.BUILD_GRANT.parent == c.EVIDENCE,
        "new_stages": (p["stage"], p["build_stage"]) == (c.STAGE, c.BUILD_STAGE)
        == ("shadow-guard-visual", "shadow-guard-build"),
        "new_publication_directories": (c.PUBLISHED, c.PUBLISHED_EVIDENCE)
        == (c.PACKET_DIR / "screenshots", c.PACKET_DIR / "evidence"),
        "thresholds_equal_calibration_rule": p["comparison"]["thresholds"]["agreement"]
        == {k: v["limit"] for k, v in p["comparison"]["calibration"]["derived"].items()},
    }


def source_checks(p):
    s = p["sources"]
    base, cand = s["baseline"]["commit"], s["candidate"]["commit"]
    kinds = [c.git("cat-file", "-t", commit).strip() for commit in (base, cand)]
    parent = c.git("rev-parse", cand + "^").strip()
    trees = [c.git("rev-parse", commit + "^{tree}").strip() for commit in (base, cand)]
    names = c.git("diff", "--name-only", base, cand).split()
    inputs = [n for n in names if any(n == r or n.startswith(r + "/") for r in p["build_inputs"])]
    return {
        "commits_exist": kinds == ["commit", "commit"],
        "candidate_parent_is_baseline": parent == base == s["candidate"]["parent"],
        "trees_match_plan": trees == [s["baseline"]["tree"], s["candidate"]["tree"]],
        "delta_matches_git": sorted(names) == sorted(s["delta"]["all_paths"]),
        "no_shader_or_cooker_change": not any(n.endswith((".wgsl", ".naga")) or "shader" in n or "cook" in n
                                              for n in names),
        "build_input_delta_is_seven_rust_files": len(inputs) == 7 and all(n.endswith(".rs") for n in inputs),
        "review_binary_delta_is_four_production_files": s["delta"]["in_review_binary"] == [
            n for n in inputs if "/src/" in n and not n.endswith("_tests.rs")] and len(s["delta"]["in_review_binary"]) == 4,
        "observed": {"parent": parent, "trees": trees, "delta": names, "build_input_delta": inputs},
    }


def fixture_checks(p, fixture_bytes):
    f = p["fixture"]
    base = blob(f["base"])
    current_lines, base_lines = fixture_bytes.decode().splitlines(), base.decode().splitlines()
    line = f["change"]["inserted_line"]
    rebuilt = current_lines[:line - 1] + current_lines[line:]
    source = fixture_bytes.decode()
    ui_input = case_body(source, "ui_input")
    old_plan = json.loads(blob(f"{FRAMEWORK}:{FRAMEWORK_VISUAL}/plan.json"))
    keys = ("frames", "ui_submissions", "pngs", "purpose", "checks")
    widget = {commit: HALOS in blob(f"{commit}:crates/incant_render_data/src/ui.rs").decode()
              for commit in (BUILT_0072, p["sources"]["baseline"]["commit"], p["sources"]["candidate"]["commit"])}
    review = re.search(r"const REVIEW: TemporalConfig = TemporalConfig \{(.*?)\};", source, re.S)
    fields = {k: float(v) for k, v in re.findall(r"(\w+): ([0-9.]+),", review[1])} if review else {}
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    env = re.search(r'const OUTPUT_ENV: &str = "(\w+)";', source)
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    return {
        "base_is_0072_fixture": hashlib.sha256(base).hexdigest() == f["base_sha256"]
        and blob(f"{FRAMEWORK}:{FRAMEWORK_VISUAL}/review/temporal_visual_review.rs") == base,
        "sha256_matches_plan": c.sha(c.HELPER_SOURCE) == f["sha256"],
        "exactly_one_inserted_line": rebuilt == base_lines and len(current_lines) == len(base_lines) + 1
        and current_lines[line - 1] == f["change"]["text"],
        "insertion_inside_ui_input": f["change"]["text"].strip() in ui_input
        and "halos" not in source.replace(f["change"]["text"], ""),
        "viewport_case_never_reaches_ui_input": "ui_input" not in case_body(source, "viewport")
        and "ui_input" not in "".join(case_body(source, n) for n in ("viewport_target", "shoot_viewport",
                                                                      "outside_delta", "border_split")),
        "field_required_by_both_sources_absent_in_0072_source": widget == {
            BUILT_0072: False, p["sources"]["baseline"]["commit"]: True, p["sources"]["candidate"]["commit"]: True},
        "tests_listed_in_order": tests == p["review"]["listed_cases"],
        "viewport_contract_checks_match_plan": case_checks(source, "viewport")
        == p["review"]["cases"]["viewport"]["checks"],
        "viewport_case_as_0072_plan": all(old_plan["review"]["cases"]["viewport"][k]
                                          == p["review"]["cases"]["viewport"][k] for k in keys),
        "output_variable_matches_plan": bool(env) and env[1] == p["review"]["output_env"],
        "size_matches_plan": bool(size) and [int(size[1]), int(size[2])] == p["review"]["size"],
        "review_config_matches_plan_and_0072": fields == {k: float(v) for k, v in p["review_config"].items()}
        and old_plan["review_config"] == p["review_config"],
        "observed": {"tests": tests, "viewport_checks": case_checks(source, "viewport"),
                     "halos_field_present": widget},
    }


def region_checks(p):
    import regions
    text = blob(p["provenance"]["diagnosis"]["derivation_output"]).decode().splitlines()
    want = text[text.index("== shadow of the card on the ground (y = 0), still pose"):]
    spec = p["comparison"]
    idx = regions.region(spec["pose"], spec["region"])
    card = regions.mask([tuple(v) for v in regions.pose(spec["pose"])["card"]], 0, spec["viewport_rect"])
    sampled = "\n".join(text)
    return {"reproduces_cascade_texels_lines": regions.cascade_lines() == want,
            "grille_sampling_uses_diagnosed_texels": "texel off 0.031495 on 0.031618" in sampled
            and "texel off 0.031740 on 0.031740" in sampled,
            "region_inside_viewport": all(regions.inside(
                [(37, 19), (922, 19), (922, 520), (37, 520)], i % 960, i // 960) for i in idx),
            "region_disjoint_from_card": not set(idx) & set(card),
            "observed": {"region_pixels": len(idx), "region_bbox": regions.bbox(idx),
                         "card_pixels": len(card), "card_bbox": regions.bbox(card),
                         "polygon": regions.pose(spec["pose"])["grille_shadow"],
                         "grille_sampling_cascade_1": regions.grille_sampling()}}


def calibration_checks(p, tmp):
    import shadowfit
    spec = p["comparison"]
    cal = spec["calibration"]
    files = {}
    wanted = {x[k] for x in cal["controls"] for k in ("on", "off") if k in x} | {x["report"] for x in cal["still_controls"]}
    for i, rel in enumerate(sorted(wanted)):
        local = Path(tmp) / f"{i}-{Path(rel).name}"
        local.write_bytes(blob(f"{cal['source_commit']}:{rel}"))
        files[rel] = str(local)
    out = shadowfit.calibrate(spec, files)
    limits = spec["thresholds"]["agreement"]
    region_keys = ("mean_gap", "blur_residual", "lowpass_max")
    consistent = {k: v for k, v in out["values"].items() if v["kind"] == "consistent"}
    mismatch = {k: v for k, v in out["values"].items() if v["kind"] == "mismatch"}
    met = {k: [v[m] <= limits[m + "_max"] for m in region_keys if m in v]
           + ([v["still_interior_max_delta"] <= limits["still_interior_max_delta_max"]]
              if "still_interior_max_delta" in v else []) for k, v in out["values"].items()}
    return {"values_recompute_exactly": out["values"] == cal["values"],
            "derived_limits_recompute_exactly": out["derived"] == cal["derived"],
            "consistent_controls_meet_every_limit": all(all(met[k]) for k in consistent),
            "mismatch_controls_fail_every_limit": all(not any(met[k]) for k in mismatch),
            "observed": {"limits": limits, "met": met}}


def pairs_checks(p, tmp):
    import pairs
    rel = "handoffs/0072-motion-history-numeric/fixed-sharpness-screenshots"
    source = Path(tmp) / "retained"
    source.mkdir()
    for name in (p["comparison"]["on"], p["comparison"]["off"]):
        (source / name).write_bytes(blob(f"{FRAMEWORK}:{rel}/{name}"))
    written = pairs.write_all({role: source for role in c.ROLES}, Path(tmp) / "pairs", p)
    spec = p["comparison"]["pairs"]
    return {"every_planned_pair_written": [w["name"] for w in written]
            == [x["name"] + ".png" for x in spec["crops"] + spec["differences"]]
            and all((Path(tmp) / "pairs" / w["name"]).is_file() for w in written),
            "observed": written}


def refusal_checks():
    if c.EVIDENCE.exists():
        return {"skipped": "the evidence directory exists, so a grant may exist"}
    before = c.git("status", "--porcelain", "--untracked-files=all")
    lock_before = c.GPU_LOCK.stat() if c.GPU_LOCK.exists() else None
    results = {}
    for command in COMMANDS:
        run = subprocess.run([sys.executable, "-I", "-B", str(c.TOOLS / command[0]), *command[1:]],
                             capture_output=True, text=True, timeout=60)
        results[" ".join(command)] = {
            "exit": run.returncode,
            "refused_for_absent_grant": run.returncode != 0 and any(m in run.stderr for m in ABSENT)}
    lock_after = c.GPU_LOCK.stat() if c.GPU_LOCK.exists() else None
    same_lock = (lock_before is None and lock_after is None) or (
        lock_before is not None and lock_after is not None
        and (lock_before.st_mtime_ns, lock_before.st_size, lock_before.st_ino)
        == (lock_after.st_mtime_ns, lock_after.st_size, lock_after.st_ino))
    return {"commands": results,
            "all_refused_for_absent_grant": all(r["refused_for_absent_grant"] for r in results.values()),
            "evidence_directory_still_absent": not c.EVIDENCE.exists(),
            "git_status_unchanged": c.git("status", "--porcelain", "--untracked-files=all") == before,
            "global_gpu_lock_untouched": same_lock}


def passed(section):
    return all(v for k, v in section.items() if k != "observed")


def main():
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--write"):
        raise SystemExit(__doc__)
    for path in sorted(c.TOOLS.glob("*.py")):
        ast.parse(path.read_text(), str(path))
    p = c.plan()
    fixture = c.HELPER_SOURCE.read_bytes()
    with tempfile.TemporaryDirectory(prefix="incant-0076-selfcheck-") as tmp:
        result = {"python": sys.version.split()[0], "plan_sha256": c.sha(c.PLAN_PATH),
                  "helper_sha256": c.sha(c.HELPER_SOURCE),
                  "tools": {path.name: c.sha(path) for path in sorted(c.TOOLS.glob("*.py"))},
                  "syntax": "all tools parse", "plan": plan_checks(p, fixture.decode()),
                  "sources": source_checks(p), "fixture": fixture_checks(p, fixture),
                  "regions": region_checks(p), "calibration": calibration_checks(p, tmp),
                  "pairs": pairs_checks(p, tmp), "refusals": refusal_checks()}
    refusals = result["refusals"]
    result["all_pass"] = (all(passed(result[k]) for k in ("plan", "sources", "fixture", "regions",
                                                          "calibration", "pairs"))
                          and all(refusals.get(k, False) for k in (
                              "all_refused_for_absent_grant", "evidence_directory_still_absent",
                              "git_status_unchanged", "global_gpu_lock_untouched")))
    text = json.dumps(result, indent=1) + "\n"
    if args:
        with open(args[1], "x") as out:
            out.write(text)
    print(text, end="")
    raise SystemExit(0 if result["all_pass"] else 1)


if __name__ == "__main__":
    main()
