#!/usr/bin/env python3
"""Preparation self-check for handoff 0068 revision 2. Standard library only; no
grant needed. Never compiles, starts a renderer, acquires a target or touches an
evidence directory (the historic 0068 directory is only read for its absence of
r2 files; the r2 directory must not exist).

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every packet Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent: first-run frames and submissions match its
   totals and fit 24 frames / 48 submissions with the stated reserve, sizes fit
   960x540, PNG names are unique, 3-6 retained pairs name real PNGs, the numeric
   test is the library suite's only skip, and the five suites total 92 cases.
3. The bound source is real: the pinned tree matches git for the pinned commit,
   and the pinned build freeze and root CPU acceptance hash match the files at
   that commit (read with git show; nothing is exported or built).
4. The Rust review helper agrees with the plan: test names, Report::open frame and
   submission limits, PNG names, frame size and output variable.
5. The acceptance rule rejects zero counters, an all-zero payload, a zero
   aggregate interval, wrong or missing labels, non-finite values and a skipped
   or not-exactly-one numeric test, and accepts equal nonzero endpoints.
6. With the r2 grant absent, every guarded command refuses with the grant
   message, creates no r2 evidence directory and leaves `git status` unchanged.
"""

import ast
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import common as c  # noqa: E402

COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"], ["build.py", "checkout"],
            ["build.py", "toolchain"], ["build.py", "tests"], ["build.py", "review"],
            ["build.py", "lint"], ["gpu.py", "list", "lib"], ["gpu.py", "numeric"],
            ["gpu.py", "suite", "lib"], ["gpu.py", "suite", "timestamp_backend_probe"],
            ["gpu.py", "review", "lit"], ["gpu.py", "review", "cascades"],
            ["gpu.py", "review", "owner_drop"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "publish"]]


def plan_checks(p):
    cases = p["review"]["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    submissions = p["numeric"]["submissions"] + sum(case["submissions"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    sizes = [case.get("size", [0, 0]) for case in cases.values()]
    reserve = totals["reserve_after_first_runs"]
    largest = max(cases.values(), key=lambda case: case["frames"])
    return {
        "first_run_frames_match_total": frames == totals["appearance_frames_first_runs"],
        "first_run_submissions_match_total": submissions == totals["numeric_submissions_first_runs"],
        "first_runs_within_caps": (frames <= budgets["appearance_frames_max"]
                                   and submissions <= budgets["numeric_submissions_max"]),
        "reserve_is_cap_minus_first_runs": (
            reserve["appearance_frames"] == budgets["appearance_frames_max"] - frames
            and reserve["numeric_submissions"] == budgets["numeric_submissions_max"] - submissions),
        "one_rerun_of_largest_case_fits_reserve": (largest["frames"] <= reserve["appearance_frames"]
                                                   and largest["submissions"] <= reserve["numeric_submissions"]),
        "sizes_within_960x540": all(w <= budgets["frame_max_width"] and h <= budgets["frame_max_height"]
                                    for w, h in sizes),
        "png_names_unique": len(names) == len(set(names)),
        "pngs_not_more_than_frames": all(len(case["pngs"]) <= case["frames"] for case in cases.values()),
        "retained_pairs_name_real_pngs": all(a in names and b in names for a, b in totals["retained_png_pairs"]),
        "retained_pairs_3_to_6": 3 <= len(totals["retained_png_pairs"]) <= 6,
        "four_prior_pairs_reused": [["lit-off-1.png", "lit-on-1.png"], ["lit-off-1.png", "lit-on-4-skipped.png"],
                                    ["sprites-text-off.png", "sprites-text-on.png"],
                                    ["viewport-off.png", "viewport-on.png"]] == totals["retained_png_pairs"][:4],
        "numeric_is_the_only_lib_skip": p["suites"]["lib"]["skip"] == [p["numeric"]["test"]],
        "other_suites_skip_nothing": all(not s["skip"] for n, s in p["suites"].items() if n != "lib"),
        "suites_total_92": sum(s["expected_ignored"] for s in p["suites"].values()) == 92,
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2")),
        "probe_binary_built": "timestamp_backend_probe" in p["builds"]["tests-render"],
        "evidence_dir_is_r2_not_historic": (str(c.EVIDENCE) == p["evidence_dir"]
                                            and c.EVIDENCE != c.HISTORIC_EVIDENCE),
    }


def source_checks(p):
    pin = p["source"]
    show = lambda path: subprocess.run(["git", "show", f"{pin['commit']}:{path}"], cwd=c.ROOT,
                                       capture_output=True, check=True).stdout
    freeze = show(pin["freeze_path"])
    acceptance = show(pin["root_cpu_acceptance"]["path"])
    return {
        "tree_matches_commit": c.git("rev-parse", pin["commit"] + "^{tree}").strip() == pin["tree"],
        "freeze_sha256_matches": hashlib.sha256(freeze).hexdigest() == pin["freeze_sha256"],
        "freeze_input_count_matches": len(json.loads(freeze)["files"]) == pin["freeze_inputs"],
        "freeze_relative_path_matches_tools": pin["freeze_path"] == c.FREEZE_RELATIVE,
        "root_cpu_acceptance_sha256_matches":
            hashlib.sha256(acceptance).hexdigest() == pin["root_cpu_acceptance"]["sha256"],
        "tested_head_is_ancestor": subprocess.run(
            ["git", "merge-base", "--is-ancestor", pin["tested_head"], pin["commit"]],
            cwd=c.ROOT).returncode == 0,
        "no_build_input_diff_from_tested_head":
            c.git("diff", "--stat", pin["tested_head"], pin["commit"], "--", *pin["build_input_roots"]) == "",
    }


def helper_checks(p):
    source = c.HELPER_SOURCE.read_text()
    cases = p["review"]["cases"]
    tests = re.findall(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(", source)
    opened = {m[0]: (int(m[1]), int(m[2]))
              for m in re.findall(r'Report::open\("(\w+)", (\d+), (\d+)\)', source)}
    matched = re.search(r"Report::open\(case, (\d+), (\d+)\)", source)
    for name in re.findall(r'matched_pair\(\s*"(\w+)",', source):
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
        "measured_frame_counts_two_submissions": "report.count_submissions(2);" in source,
        "observed": {"tests": tests, "limits": opened, "pngs": sorted(pngs)},
    }


def acceptance_checks():
    """Synthetic samples through analyze's acceptance rule; CPU only."""
    expected = analyze.expected_groups({"model_draw_calls": 1, "sprite_entities": 0,
                                        "tilemap_entities": 0, "text_glyph_quads": 0}, 2, 1)
    span = lambda start, end: {"start_ticks": start, "end_ticks": end, "nanoseconds": float(end - start)}
    good = {"InitializeScene": span(100, 120), "AssignLights": span(121, 130), "Shadows": span(131, 300),
            "Models": span(301, 500), "Sprites": None, "Text": None, "Display": span(501, 520)}
    frame = span(100, 520)
    quantized = dict(good, AssignLights=span(125, 125))
    zero = {name: (span(0, 0) if value else None) for name, value in good.items()}
    no_shadows = dict(good, Shadows=None)
    extra_text = dict(good, Text=span(510, 515))
    nan = dict(good, Models={"start_ticks": 301, "end_ticks": 500, "nanoseconds": float("nan")})
    verdict = lambda spans, frame_span: analyze.sample_review(spans, frame_span, 1.0, expected)["accepted"]
    unshadowed = analyze.expected_groups({"model_draw_calls": 1, "sprite_entities": 0,
                                          "tilemap_entities": 0, "text_glyph_quads": 0}, 0, 0)
    sample = {"sequence": 1, "timestamp_period_ns": 1.0, "frame": span(100, 520),
              "passes": [span(100, 120), None, None, span(301, 500), None, None, span(501, 520)]}
    one = {"exit": 0, "timed_out": False, "libtest": {}, "exactly_one_test_ran": True,
           "test_passed": True, "samples": [sample, dict(sample, sequence=2)]}
    skipped = dict(one, exactly_one_test_ran=False, test_passed=False)
    zeros = dict(one, samples=[dict(sample, frame=span(0, 0), passes=[span(0, 0) if x else None
                                                                       for x in sample["passes"]])] * 2)
    results = {
        "expected_shadows_need_shadowing_light": expected["Shadows"] and not unshadowed["Shadows"],
        "nonzero_ordered_sample_accepted": verdict(good, frame),
        "equal_nonzero_endpoints_accepted": verdict(quantized, frame),
        "all_zero_payload_rejected": not verdict(zero, span(0, 0)),
        "zero_aggregate_rejected": not verdict(good, span(300, 300)),
        "missing_expected_group_rejected": not verdict(no_shadows, frame),
        "unexpected_group_rejected": not verdict(extra_text, frame),
        "non_finite_value_rejected": not verdict(nan, frame),
        "exactly_one_numeric_test_accepted": analyze.numeric_review(one)["accepted"],
        "skipped_numeric_test_rejected": not analyze.numeric_review(skipped)["accepted"],
        "zero_numeric_samples_rejected": not analyze.numeric_review(zeros)["accepted"],
    }
    return results


def refusal_checks():
    if c.EVIDENCE.exists():
        return {"skipped": "the r2 evidence directory exists, so the grant may exist"}
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
              "syntax": "all tools parse", "plan": plan_checks(p), "source": source_checks(p),
              "helper": helper_checks(p), "acceptance": acceptance_checks(),
              "refusals": refusal_checks()}
    result["all_pass"] = (all(result["plan"].values()) and all(result["source"].values())
                          and all(v for k, v in result["helper"].items() if k != "observed")
                          and all(result["acceptance"].values())
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
