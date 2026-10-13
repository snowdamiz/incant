#!/usr/bin/env python3
"""Preparation self-check for handoff 0072's fixed-sharpness comparison review.
Standard library only; no grant needed. Never compiles, starts a renderer,
acquires a target or touches the stage evidence directory or the global GPU lock.

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every stage Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent and within the brief's budgets: 105 visual
   (world + UI) submissions within 120, 102 + 150 numeric = 252, 357 planned within
   372, 16 of 16 PNGs at 960x540.
3. Comparability with the accepted coverage baseline (visual tree b1327f3d): the
   fixture is that fixture byte for byte plus one appended case (viewport); its
   seven cases keep their frames, UI submissions, PNG names, purposes and checks;
   the baseline fixture still differs from the original only at line 1418.
4. The Rust fixture agrees with the plan: test names in order, Report::open
   limits, PNG names, frame size, output variable, the explicit config and each
   case's exact contract-check names.
5. With both grants absent, every guarded command refuses with its grant
   message, creates no evidence directory, leaves `git status` unchanged and
   does not touch the global native-gpu.lock.
"""

import ast
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

ORIGINAL = "9009f088ac230e31185e8ce64183ca93878e9444"
BASELINE_TREE = "b1327f3d9abbaf10f4ba8701dc4ef81231b6d60e"  # visual tree of the accepted coverage run
NUMERIC = {"resolve": "temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion",
           "coverage": "temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection"}
COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"], ["build.py", "checkout"],
            ["build.py", "toolchain"], ["build.py", "libtests"], ["build.py", "review"],
            ["build.py", "lint"], ["gpu.py", "list", "lib"], ["gpu.py", "numeric", "resolve"],
            ["gpu.py", "numeric", "coverage"],
            ["gpu.py", "list", "review"], ["gpu.py", "review", "probe"],
            ["gpu.py", "review", "stationary"], ["gpu.py", "release"],
            ["gpu.py", "review", "viewport"], ["analyze.py", "check"], ["analyze.py", "compare"],
            ["analyze.py", "publish"]]
ABSENT = ("build-ready.json is absent", "review-ready.json is absent")


def plan_checks(p):
    spec = p["review"]
    cases = spec["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    ui = sum(case["ui_submissions"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    tests = p["numeric"]["tests"]
    numeric = sum(t["submissions"] for t in tests)
    config = p["review_config"]
    return {
        "frames_match_total": frames == totals["frames"] == 96,
        "ui_match_total": ui == totals["ui_submissions"] == 9,
        "visual_105_within_120": frames + ui == totals["review_submissions"] == 105
        and budgets["visual_submissions_max"] == 120,
        "numeric_102_plus_150": [t["submissions"] for t in tests] == [102, 150]
        and numeric == totals["numeric_submissions"] == budgets["numeric_submissions_max"] == 252,
        "planned_357_within_372": (frames + ui + numeric == totals["submissions"] == budgets["planned_submissions"]
                                   == 357 and budgets["submissions_max"] == 372),
        "pngs_16_of_16": len(names) == totals["pngs"] == 16 and budgets["pngs_max"] == 16,
        "reserves_match": (totals["reserve_submissions"] == 372 - 357 == totals["reserve_visual_submissions"]
                           and totals["reserve_pngs"] == 0),
        "gpu_children_at_most_120_s": all(budgets[k] <= 120 for k in (
            "listing_timeout_s", "numeric_timeout_s", "review_case_timeout_s")),
        "png_names_unique": len(names) == len(set(names)),
        "size_960x540": spec["size"] == [960, 540] and budgets["frame_max_width"] == 960
        and budgets["frame_max_height"] == 540,
        "one_attempt_each": spec["max_runs_per_case"] == 1 and p["numeric"]["max_runs"] == 1,
        "order_names_every_case": sorted(spec["order"]) == sorted(cases),
        "every_case_lists_checks": all(case.get("checks") for case in cases.values()),
        "no_whole_suite": "suites" not in p and "tests-render" not in p["builds"]
        and "tests-host" not in p["builds"],
        "exact_numeric_tests": {t["id"]: t["test"] for t in tests} == NUMERIC,
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2"))
        and budgets["compile_jobs"] == 2,
        "review_config_in_validated_ranges": (
            0 <= config["history_weight"] <= 0.99 and 0 <= config["relative_depth"] <= 0.1
            and 0 <= config["absolute_depth"] <= 1 and 1e-6 <= config["reactive_threshold"] <= 65504
            and 0 <= config["reactive_strength"] <= 1),
        "expected_backend_metal": p["expected_backend"] == "Metal",
        "new_evidence_namespace": p["evidence_namespace"] == str(c.EVIDENCE)
        and c.EVIDENCE.name == "0072-fixed-sharpness-001" and c.GRANT.parent == c.EVIDENCE
        and c.BUILD_GRANT.parent == c.EVIDENCE,
        "new_stages": (p["stage"], p["build_stage"]) == (c.STAGE, c.BUILD_STAGE)
        == ("fixed-sharpness-visual", "fixed-sharpness-build"),
        "new_publication_directories": (c.PUBLISHED.name, c.PUBLISHED_EVIDENCE.name)
        == ("fixed-sharpness-screenshots", "fixed-sharpness-evidence"),
        "comparison_baseline_and_candidate": p["comparison"]["baseline"]["visual_tree"] == BASELINE_TREE
        and p["comparison"]["candidate_commit"] == p["source_pinning"]["expected_commit"]
        == "a143b5c0f537bd35111e40e99626f40318a385ac",
    }


def comparability_checks(p):
    original_plan = json.loads(c.git("show", f"{ORIGINAL}:{c.VISUAL_RELATIVE}/plan.json"))
    original_fixture = c.git("show", f"{ORIGINAL}:{c.VISUAL_RELATIVE}/review/temporal_visual_review.rs")
    baseline_plan = json.loads(c.git("show", f"{BASELINE_TREE}:plan.json"))
    baseline_fixture = subprocess.run(["git", "cat-file", "blob", f"{BASELINE_TREE}:review/temporal_visual_review.rs"],
                                      cwd=c.ROOT, capture_output=True, check=True).stdout
    current = c.HELPER_SOURCE.read_bytes()
    appended = current[len(baseline_fixture):].decode()
    keys = ("frames", "ui_submissions", "pngs", "purpose", "checks")
    baseline_cases = baseline_plan["review"]["cases"]
    same_as_baseline = {name: all(p["review"]["cases"].get(name, {}).get(k) == case.get(k) for k in keys)
                        for name, case in baseline_cases.items()}
    same_as_original = {name: all(case.get(k) == original_plan["review"]["cases"].get(name, {}).get(k)
                                  for k in ("frames", "ui_submissions", "pngs", "purpose"))
                        for name, case in baseline_cases.items()}
    base_text = baseline_fixture.decode()
    changed = [(i, a, b) for i, (a, b) in enumerate(zip(original_fixture.splitlines(),
                                                        base_text.splitlines()), 1) if a != b]
    return {
        "fixture_extends_baseline_exactly": current.startswith(baseline_fixture),
        "appended_block_adds_only_viewport_case": re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", appended)
        == ["viewport"],
        "baseline_cases_unchanged": all(same_as_baseline.values())
        and sorted(p["review"]["cases"]) == sorted([*baseline_cases, "viewport"]),
        "baseline_cases_as_original": all(same_as_original.values()),
        "same_review_config": p["review_config"] == baseline_plan["review_config"] == original_plan["review_config"],
        "baseline_fixture_only_line_1418_from_original": (
            len(original_fixture.splitlines()) == len(base_text.splitlines())
            and [i for i, _, _ in changed] == [1418]),
        "observed": {"baseline_case_equality": same_as_baseline, "appended_bytes": len(appended.encode()),
                     "baseline_fixture_bytes": len(baseline_fixture)},
    }


def case_checks(source, name):
    body = re.search(r"fn " + name + r"\(\) \{(.*?)(?=\n#\[test\]|\Z)", source, re.S)
    block = re.search(r"let checks = \[(.*?)\n    \];", body[1], re.S) if body else None
    return re.findall(r'\(\s*"([a-z0-9_]+)",', block[1]) if block else []


def helper_checks(p):
    source = c.HELPER_SOURCE.read_text()
    spec = p["review"]
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    opened = {m[0]: (int(m[1]), int(m[2]))
              for m in re.findall(r'Report::open\("(\w+)", (\d+), (\d+)\)', source)}
    pngs = re.findall(r'report\.png\("([\w-]+\.png)"', source)
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    env = re.search(r'const OUTPUT_ENV: &str = "(\w+)";', source)
    review = re.search(r"const REVIEW: TemporalConfig = TemporalConfig \{(.*?)\};", source, re.S)
    fields = {k: float(v) for k, v in re.findall(r"(\w+): ([0-9.]+),", review[1])} if review else {}
    checks = {name: case_checks(source, name) for name in spec["order"]}
    return {
        "tests_match_plan_in_order": tests == spec["order"],
        "limits_match_plan": all(opened.get(name) == (case["frames"], case["ui_submissions"])
                                 for name, case in spec["cases"].items()),
        "png_names_match_plan": sorted(pngs) == sorted(n for case in spec["cases"].values() for n in case["pngs"]),
        "size_matches_plan": bool(size) and [int(size[1]), int(size[2])] == spec["size"],
        "output_variable_matches_plan": bool(env) and env[1] == spec["output_env"],
        "review_config_matches_plan": fields == {k: float(v) for k, v in p["review_config"].items()},
        "contract_checks_match_plan": all(checks[name] == spec["cases"][name]["checks"] for name in spec["order"]),
        "observed": {"tests": tests, "limits": opened, "pngs": pngs, "review_config": fields, "checks": checks},
    }


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


def main():
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--write"):
        raise SystemExit(__doc__)
    for path in sorted(c.TOOLS.glob("*.py")):
        ast.parse(path.read_text(), str(path))
    p = c.plan()
    result = {"python": sys.version.split()[0], "plan_sha256": c.sha(c.PLAN_PATH),
              "helper_sha256": c.sha(c.HELPER_SOURCE),
              "tools": {path.name: c.sha(path) for path in sorted(c.TOOLS.glob("*.py"))},
              "syntax": "all tools parse", "plan": plan_checks(p),
              "comparability": comparability_checks(p), "helper": helper_checks(p),
              "refusals": refusal_checks()}
    refusals = result["refusals"]
    result["all_pass"] = (all(result["plan"].values())
                          and all(v for k, v in result["comparability"].items() if k != "observed")
                          and all(v for k, v in result["helper"].items() if k != "observed")
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
