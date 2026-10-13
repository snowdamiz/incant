#!/usr/bin/env python3
"""Preparation self-check for handoff 0072 stage B. Standard library only; no
grant needed. Never compiles, starts a renderer, acquires a target or touches
the stage evidence directory or the global GPU lock.

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every stage Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent and within the brief's budgets.
3. The Rust fixture agrees with the plan: test names and order, Report::open
   limits, PNG names, frame size, output variable and the explicit config.
4. With the grant absent, every guarded command refuses with the grant message,
   creates no evidence directory, leaves `git status` unchanged and does not
   touch the global native-gpu.lock.
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

COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"], ["build.py", "checkout"],
            ["build.py", "toolchain"], ["build.py", "tests"], ["build.py", "review"],
            ["build.py", "lint"], ["gpu.py", "list", "lib"], ["gpu.py", "list", "review"],
            ["gpu.py", "suite", "lib"], ["gpu.py", "review", "probe"],
            ["gpu.py", "review", "stationary"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "publish"]]


def plan_checks(p):
    cases = p["review"]["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    ui = sum(case["ui_submissions"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    config = p["review_config"]
    return {
        "frames_match_total": frames == totals["frames"],
        "ui_match_total": ui == totals["ui_submissions"],
        "submissions_match_total": frames + ui == totals["new_submissions"],
        "submissions_within_96": frames + ui <= budgets["new_submissions_max"] == 96,
        "pngs_match_total": len(names) == totals["pngs"],
        "pngs_within_16": len(names) <= budgets["pngs_max"] == 16,
        "reserves_match": (totals["reserve_submissions"] == 96 - frames - ui
                           and totals["reserve_pngs"] == 16 - len(names)),
        "png_names_unique": len(names) == len(set(names)),
        "pngs_not_more_than_frames": all(len(case["pngs"]) <= case["frames"] for case in cases.values()),
        "size_within_960x540": p["review"]["size"][0] <= budgets["frame_max_width"] == 960
        and p["review"]["size"][1] <= budgets["frame_max_height"] == 540,
        "one_run_per_case": p["review"]["max_runs_per_case"] == 1,
        "regression_total_96": sum(s["expected_ignored_at_reviewed_source"]
                                   for s in p["suites"].values()) == 96,
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2"))
        and budgets["compile_jobs"] == 2,
        "review_config_in_validated_ranges": (
            0 <= config["history_weight"] <= 0.99 and 0 <= config["relative_depth"] <= 0.1
            and 0 <= config["absolute_depth"] <= 1 and 1e-6 <= config["reactive_threshold"] <= 65504
            and 0 <= config["reactive_strength"] <= 1),
        "expected_backend_metal": p["expected_backend"] == "Metal",
    }


def helper_checks(p):
    source = c.HELPER_SOURCE.read_text()
    cases = p["review"]["cases"]
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    opened = {m[0]: (int(m[1]), int(m[2]))
              for m in re.findall(r'Report::open\("(\w+)", (\d+), (\d+)\)', source)}
    pngs = re.findall(r'report\.png\("([\w-]+\.png)"', source)
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    env = re.search(r'const OUTPUT_ENV: &str = "(\w+)";', source)
    review = re.search(r"const REVIEW: TemporalConfig = TemporalConfig \{(.*?)\};", source, re.S)
    fields = {k: float(v) for k, v in re.findall(r"(\w+): ([0-9.]+),", review[1])} if review else {}
    return {
        "tests_match_plan_in_order": tests == list(cases),
        "limits_match_plan": all(opened.get(name) == (case["frames"], case["ui_submissions"])
                                 for name, case in cases.items()),
        "png_names_match_plan": sorted(pngs) == sorted(n for case in cases.values() for n in case["pngs"]),
        "size_matches_plan": bool(size) and [int(size[1]), int(size[2])] == p["review"]["size"],
        "output_variable_matches_plan": bool(env) and env[1] == p["review"]["output_env"],
        "review_config_matches_plan": fields == {k: float(v) for k, v in p["review_config"].items()},
        "observed": {"tests": tests, "limits": opened, "pngs": pngs, "review_config": fields},
    }


def refusal_checks():
    if c.EVIDENCE.exists():
        return {"skipped": "the evidence directory exists, so the grant may exist"}
    before = c.git("status", "--porcelain", "--untracked-files=all")
    lock_before = c.GPU_LOCK.stat() if c.GPU_LOCK.exists() else None
    results = {}
    for command in COMMANDS:
        run = subprocess.run([sys.executable, "-I", "-B", str(c.TOOLS / command[0]), *command[1:]],
                             capture_output=True, text=True, timeout=60)
        results[" ".join(command)] = {
            "exit": run.returncode,
            "refused_for_absent_grant": run.returncode != 0
            and "review-ready.json is absent" in run.stderr}
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
              "syntax": "all tools parse", "plan": plan_checks(p), "helper": helper_checks(p),
              "refusals": refusal_checks()}
    refusals = result["refusals"]
    result["all_pass"] = (all(result["plan"].values())
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
