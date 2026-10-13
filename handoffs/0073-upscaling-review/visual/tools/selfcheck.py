#!/usr/bin/env python3
"""Preparation self-check for the 0073 upscaling visual stage. Standard library only;
no grant needed. Never compiles, starts a renderer, acquires a target, takes a
lock or touches the evidence directory.

Adapted from handoff 0068's committed, root-reviewed tools/selfcheck.py.

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every tool parses (ast only; no bytecode is written).
2. plan.json is consistent: case frames are scene plus UI submissions and add up
   to its totals within 96; PNG names are unique, case-prefixed and within 16;
   every case lists unique contract checks; builds are locked, offline, ci, -j2,
   feature-free and refuse cooked-* and native-metalfx; no source is pinned in
   the plan (only root's build grant names one); both grants are documented; the
   order covers every case once.
3. The Rust helper agrees with the plan: test names, Report::open limits, PNG
   names and contract check names per case, the 960x540 size, the output
   variable, the review policy, and the constants analyze.py mirrors (UI
   rectangles, viewport, edge threshold).
4. With no grant present, every guarded command refuses naming the grant it
   needs (build-ready.json for lease/build steps, review-ready.json for GPU and
   analysis steps), creates no evidence directory, leaves `git status` unchanged
   and leaves the shared native-gpu.lock untouched.
"""

import ast
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import common as c  # noqa: E402

BUILD_COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"], ["build.py", "checkout"],
                  ["build.py", "toolchain"], ["build.py", "review"], ["build.py", "lint"]]
GPU_COMMANDS = [["gpu.py", "list"], ["gpu.py", "review", "probe"], ["gpu.py", "review", "stationary"],
                ["gpu.py", "release"], ["analyze.py", "check"], ["analyze.py", "publish"]]


def plan_checks(p):
    cases = p["review"]["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    flags = p["builds"]["common_flags"]
    return {
        "case_frames_are_scene_plus_ui": all(
            case["frames"] == case["scene_submissions"] + case["ui_submissions"]
            for case in cases.values()),
        "frames_match_total": frames == totals["frames"],
        "frames_within_96": frames <= budgets["frames_max"] == 96,
        "reserve_frames_consistent": totals["reserve_frames"] == budgets["frames_max"] - frames,
        "png_names_unique": len(names) == len(set(names)),
        "png_names_case_prefixed": all(name.startswith(case + "-") for case, spec in cases.items()
                                       for name in spec["pngs"]),
        "pngs_match_total_within_16": len(names) == totals["pngs"] <= budgets["pngs_max"] == 16,
        "reserve_pngs_consistent": totals["reserve_pngs"] == budgets["pngs_max"] - len(names),
        "png_size_960x540": budgets["png_size"] == [960, 540],
        "one_run_per_case": p["review"]["max_runs_per_case"] == 1,
        "order_covers_cases_once": sorted(p["review"]["order"]) == sorted(cases),
        "builds_locked_offline_ci_j2": all(flag in flags for flag in ("--locked", "--offline", "ci", "-j2")),
        "builds_feature_free": not any("feature" in arg for arg in
                                       p["builds"]["review"] + p["builds"]["lint"] + flags),
        "review_build_names_binary": p["builds"]["review"][-3:] == ["--test", p["review"]["binary"], "--no-run"],
        "builds_refuse_cooked_and_metalfx": sorted(p["builds"]["forbidden_feature_prefixes"])
        == ["cooked-", "native-metalfx"],
        "backend_env_metal": p["review"]["backend_env"] == {"INCANT_EXPECT_GPU_BACKEND": "Metal"},
        "case_checks_nonempty_unique": all(case["checks"] and len(case["checks"]) == len(set(case["checks"]))
                                           for case in cases.values()),
        "no_source_pinned_in_plan": not ({"producer", "producer_manifest", "source_commit"} & set(p)),
        "build_grant_documented": {"source_commit", "source_freeze", "visual_tree", "target"}
        <= set(p["grants"]["build"]["fields"]),
        "gpu_grant_documented": {"visual_tree", "build_record_sha256"} <= set(p["grants"]["gpu"]["fields"]),
        "build_inputs_cover_cargo_sources": sorted(p["build_inputs"]) == sorted(
            ["crates", "Cargo.toml", "Cargo.lock", "third_party", "tools/cargo", "rust-toolchain.toml"]),
    }


def rust_const(source, name):
    match = re.search(r"const " + name + r": [^=]+= ([^;]+);", source)
    return match[1] if match else None


def numbers(text):
    return [float(v) for v in re.findall(r"-?\d+(?:\.\d*)?", text or "")]


def helper_checks(p):
    source = c.HELPER_SOURCE.read_text()
    cases = p["review"]["cases"]
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    opened = {m[0]: int(m[1]) for m in re.findall(r'Report::open\("(\w+)", (\d+)\)', source)}
    bodies = re.split(r"\n#\[test\]", source)[1:]
    pngs, checks = {}, {}
    for body in bodies:
        name = re.search(r"fn (\w+)\(", body)[1]
        pngs[name] = re.findall(r'report\.png\("([\w-]+\.png)"', body)
        block = re.search(r"let checks = \[(.*?)\n    \];", body, re.S)
        checks[name] = re.findall(r'\(\s*"([a-z0-9_]+)",', block[1]) if block else []
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    review = rust_const(source, "REVIEW") or ""
    policy = p["policy"]
    viewport = rust_const(source, "VIEWPORT") or ""
    return {
        "tests_match_plan": sorted(tests) == sorted(case["test"] for case in cases.values()),
        "limits_match_plan": all(opened.get(name) == case["frames"] for name, case in cases.items()),
        "png_names_per_case_match_plan": all(sorted(pngs.get(name, [])) == sorted(case["pngs"])
                                             for name, case in cases.items()),
        "contract_checks_per_case_match_plan": all(checks.get(name) == case["checks"]
                                                   for name, case in cases.items()),
        "size_960x540": size is not None and [int(size[1]), int(size[2])] == [960, 540],
        "output_variable_matches_plan": f'const OUTPUT_ENV: &str = "{p["review"]["output_env"]}";' in source,
        "policy_matches_plan": numbers(review) == [policy[k] for k in (
            "history_weight", "relative_depth", "absolute_depth", "reactive_threshold",
            "reactive_strength")],
        "analyze_ui_button_matches": numbers(rust_const(source, "UI_BUTTON")) == list(analyze.UI_BUTTON),
        "analyze_ui_label_matches": numbers(rust_const(source, "UI_LABEL")) == list(analyze.UI_LABEL),
        "analyze_ui_interior_matches": numbers(rust_const(source, "UI_INTERIOR")) == list(analyze.UI_INTERIOR),
        "analyze_viewport_matches": numbers(viewport)[:4] == list(analyze.VIEWPORT_RECT)
        and numbers(viewport)[:4] == p["review"]["cases"]["transitions"]["viewport"]["rect"],
        "analyze_edge_threshold_matches": numbers(rust_const(source, "EDGE_THRESHOLD")) == [analyze.EDGE_THRESHOLD],
        "observed": {"tests": tests, "limits": opened, "pngs": pngs, "checks": checks},
    }


def refusal_checks():
    if c.EVIDENCE.exists():
        return {"skipped": "the evidence directory exists, so the grant may exist"}
    before = c.git("status", "--porcelain", "--untracked-files=all")
    lock_before = c.GLOBAL_LOCK.stat() if c.GLOBAL_LOCK.exists() else None
    results = {}
    for needed, commands in (("build-ready.json", BUILD_COMMANDS), ("review-ready.json", GPU_COMMANDS)):
        for command in commands:
            run = subprocess.run([sys.executable, "-I", "-B", str(c.TOOLS / command[0]), *command[1:]],
                                 capture_output=True, text=True, timeout=60)
            results[" ".join(command)] = {
                "exit": run.returncode, "needs": needed,
                "refused_for_absent_grant": run.returncode != 0 and f"{needed} is absent" in run.stderr}
    lock_after = c.GLOBAL_LOCK.stat() if c.GLOBAL_LOCK.exists() else None
    same_lock = (lock_before is None and lock_after is None) or (
        lock_before is not None and lock_after is not None
        and (lock_before.st_ino, lock_before.st_mtime_ns, lock_before.st_size)
        == (lock_after.st_ino, lock_after.st_mtime_ns, lock_after.st_size))
    return {"commands": results,
            "all_refused_for_absent_grant": all(r["refused_for_absent_grant"] for r in results.values()),
            "evidence_directory_still_absent": not c.EVIDENCE.exists(),
            "git_status_unchanged": c.git("status", "--porcelain", "--untracked-files=all") == before,
            "global_gpu_lock_untouched": same_lock}


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
    refusals = result["refusals"]
    result["all_pass"] = (all(result["plan"].values())
                          and all(v for k, v in result["helper"].items() if k != "observed")
                          and refusals.get("all_refused_for_absent_grant", False)
                          and refusals.get("evidence_directory_still_absent", False)
                          and refusals.get("git_status_unchanged", False)
                          and refusals.get("global_gpu_lock_untouched", False))
    text = json.dumps(result, indent=1) + "\n"
    if args:
        with open(args[1], "x") as out:
            out.write(text)
    print(text, end="")


if __name__ == "__main__":
    main()
