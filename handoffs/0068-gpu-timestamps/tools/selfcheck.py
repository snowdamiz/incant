#!/usr/bin/env python3
"""Preparation self-check for handoff 0068. Standard library only; no grant needed.
Never compiles, starts a renderer, acquires a target or touches the evidence directory.

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every packet Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent: per-case frames/submissions add up to its
   totals, all runs including permitted reruns fit 24 frames and 48 submissions,
   sizes fit 960x540, PNG names are unique and retained pairs name real PNGs, and
   the numeric test is the lib suite's only skip.
3. The Rust review helper agrees with the plan: test names, Report::open frame and
   submission limits, PNG names, frame size and output variable.
4. With the grant absent, every guarded command refuses with the grant message,
   creates no evidence directory and leaves `git status` unchanged.
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
            ["gpu.py", "list", "lib"], ["gpu.py", "numeric"], ["gpu.py", "suite", "lib"],
            ["gpu.py", "review", "lit"], ["gpu.py", "review", "owner_drop"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "publish"]]


def plan_checks(p):
    cases = p["review"]["cases"]
    runs = p["review"]["max_runs_per_case"]
    frames = sum(case["frames"] for case in cases.values())
    submissions = p["numeric"]["submissions"] + sum(case["submissions"] for case in cases.values())
    rerun_submissions = p["numeric"]["submissions"] * p["numeric"]["max_runs"] + runs * (
        submissions - p["numeric"]["submissions"])
    names = [name for case in cases.values() for name in case["pngs"]]
    budgets, totals = p["budgets"], p["totals"]
    sizes = [case.get("size", [0, 0]) for case in cases.values()]
    return {
        "first_run_frames_match_total": frames == totals["appearance_frames_first_runs"],
        "rerun_frames_match_total": frames * runs == totals["appearance_frames_with_all_reruns"],
        "rerun_frames_within_budget": frames * runs <= budgets["appearance_frames_max"],
        "first_run_submissions_match_total": submissions == totals["numeric_submissions_first_runs"],
        "rerun_submissions_match_total": rerun_submissions == totals["numeric_submissions_with_all_reruns"],
        "rerun_submissions_within_budget": rerun_submissions <= budgets["numeric_submissions_max"],
        "sizes_within_960x540": all(w <= budgets["frame_max_width"] and h <= budgets["frame_max_height"]
                                    for w, h in sizes),
        "png_names_unique": len(names) == len(set(names)),
        "pngs_not_more_than_frames": all(len(case["pngs"]) <= case["frames"] for case in cases.values()),
        "retained_pairs_name_real_pngs": all(a in names and b in names for a, b in totals["retained_png_pairs"]),
        "retained_pairs_3_to_6": 3 <= len(totals["retained_png_pairs"]) <= 6,
        "numeric_is_the_only_lib_skip": p["suites"]["lib"]["skip"] == [p["numeric"]["test"]],
        "other_suites_skip_nothing": all(not s["skip"] for n, s in p["suites"].items() if n != "lib"),
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2")),
        "reviewed_base_ignored_total_90": sum(s["expected_ignored_at_reviewed_base"]
                                              for s in p["suites"].values()) == 90,
    }


def helper_checks(p):
    source = c.HELPER_SOURCE.read_text()
    cases = p["review"]["cases"]
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    opened = {m[0]: (int(m[1]), int(m[2]))
              for m in re.findall(r'Report::open\("(\w+)", (\d+), (\d+)\)', source)}
    matched = re.search(r"Report::open\(case, (\d+), (\d+)\)", source)
    paired = re.findall(r'matched_pair\(\s*"(\w+)",', source)
    for name in paired:
        opened[name] = (int(matched[1]), int(matched[2]))
    pngs = set(re.findall(r'report\.png\("([\w-]+\.png)"', source))
    for pair in re.findall(r'\[\s*"([\w-]+\.png)",\s*"([\w-]+\.png)",?\s*\]', source):
        pngs.update(pair)
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    return {
        "tests_match_plan": sorted(tests) == sorted(case["test"] for case in cases.values()),
        "limits_match_plan": all(opened.get(name) == (case["frames"], case["submissions"])
                                 for name, case in cases.items()),
        "png_names_match_plan": pngs == {n for case in cases.values() for n in case["pngs"]},
        "size_matches_plan": all(case.get("size", [int(size[1]), int(size[2])])
                                 == [int(size[1]), int(size[2])] for case in cases.values()),
        "output_variable_matches_plan": p["review"]["output_env"] in source,
        "observed": {"tests": tests, "limits": opened, "pngs": sorted(pngs)},
    }


def refusal_checks():
    if c.EVIDENCE.exists():
        return {"skipped": "the evidence directory exists, so the grant may exist"}
    before = c.git("status", "--porcelain", "--untracked-files=all")
    results = {}
    for command in COMMANDS:
        run = subprocess.run([sys.executable, "-I", "-B", str(c.TOOLS / command[0]), *command[1:]],
                             capture_output=True, text=True, timeout=60)
        results[" ".join(command)] = {
            "exit": run.returncode,
            "refused_for_absent_grant": run.returncode != 0
            and "review-ready.json is absent" in run.stderr}
    return {"commands": results,
            "all_refused_for_absent_grant": all(r["refused_for_absent_grant"] for r in results.values()),
            "evidence_directory_still_absent": not c.EVIDENCE.exists(),
            "git_status_unchanged": c.git("status", "--porcelain", "--untracked-files=all") == before}


def main():
    args = sys.argv[1:]
    if args not in ([], ["--write", args[-1] if args else ""]):
        raise SystemExit(__doc__)
    for path in sorted(c.TOOLS.glob("*.py")):
        ast.parse(path.read_text(), str(path))
    p = c.plan()
    result = {"python": sys.version.split()[0], "plan_sha256": c.sha(c.PLAN_PATH),
              "helper_sha256": c.sha(c.HELPER_SOURCE),
              "tools": {path.name: c.sha(path) for path in sorted(c.TOOLS.glob("*.py"))},
              "syntax": "all tools parse", "plan": plan_checks(p), "helper": helper_checks(p),
              "refusals": refusal_checks()}
    result["all_pass"] = (all(result["plan"].values())
                          and all(v for k, v in result["helper"].items() if k != "observed")
                          and result["refusals"].get("all_refused_for_absent_grant", False)
                          and result["refusals"].get("evidence_directory_still_absent", False)
                          and result["refusals"].get("git_status_unchanged", False))
    text = json.dumps(result, indent=1) + "\n"
    if args:
        with open(args[1], "x") as out:
            out.write(text)
    print(text, end="")


if __name__ == "__main__":
    main()
