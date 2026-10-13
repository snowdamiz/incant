#!/usr/bin/env python3
"""Preparation self-check for handoff 0074 stage B. Standard library only; no
grant needed. Never compiles, starts a renderer, acquires a target or touches
the stage evidence directory or the global GPU lock.

Adapted from handoff 0072's committed visual/tools/selfcheck.py (visual tree
ac8a48de).

  python3 -I -B selfcheck.py [--write <new-output.json>]

1. Every stage Python tool parses (ast only; no bytecode is written).
2. plan.json is internally consistent, within the brief's 128-submission and
   20-PNG budgets, and keeps 0072's review policy unchanged.
3. An independent model of the frame, upscaling and native plans classifies
   every planned step exactly as the plan expects (native, native or device
   Scale fallback, FractionalViewport) and admits every portable footprint.
4. The Rust fixture agrees with the plan: test names and order, Report::open
   limits, PNG names and required check names per case, frame size, output
   variable, configs, viewports, preferences, planned internal sizes and the
   jitter table of the source.
5. Every item the plan lists as reused is byte-identical to 0072's reviewed
   fixture blob; the adapted tools descend from 0072's reviewed tool blobs.
6. The grant policy refuses absent, superseded or ill-formed sources (pure).
7. rustfmt (edition 2024, stdin to stdout, no Cargo or target) leaves the
   fixture byte-identical.
8. Root's announced source (read-only git objects, no export): the committed
   build freeze has its announced SHA-256 and the {commit, files} shape
   build.py accepts, the commit has its tree, the freeze covers the plan's
   build inputs exactly by path, size and blob SHA-256, and the fixture's
   jitter copy equals that commit's table.
9. With the grant absent, every guarded command refuses with the grant message,
   creates no evidence directory, leaves `git status` unchanged and does not
   touch the global native-gpu.lock.
"""

import ast
import difflib
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402
import common as c  # noqa: E402

COMMANDS = [["lease.py", "acquire"], ["lease.py", "release"], ["build.py", "checkout"],
            ["build.py", "toolchain"], ["build.py", "review"], ["build.py", "lint"],
            ["gpu.py", "list", "review"], ["gpu.py", "review", "stationary"],
            ["gpu.py", "review", "viewport"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "publish"]]
CASE_ORDER = ["stationary", "motion", "excluded", "transitions", "viewport"]
PREPARED_SOURCE = "4bc810fe0daf238f219801de76b06f19e4daf826"
COMBINED_HEAD_AT_PREPARATION = "0c890514f84c09cb98358ba6d7fa9431140b5b7c"
# Announced in root's helper-acceptance revision; the grant itself comes later.
ANNOUNCED = {"commit": "535003d766bcc1386fd2d2f454c83808a0fcc4c8",
             "tree": "e98b24847c28f3491f56e38edb051e018748eff7",
             "freeze": "/Users/andreyyurlov/.codex/incant-evidence/temporal-metalfx-union-001/committed-build-source.json",
             "freeze_sha256": "bd8295b0eb3174b96eb5ff73b0e0e7c464c6c5d620fd66df37bd59d3eafe56a0"}


def git_show(spec):
    run = subprocess.run(["git", "show", spec], cwd=c.ROOT, capture_output=True, text=True)
    return run.stdout if run.returncode == 0 else None


# ------------------------------------------------------------------ plan

def plan_checks(p):
    cases = p["review"]["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    ui = sum(case["ui_submissions"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    base = json.loads(git_show(p["base_fixture"]["visual_tree"] + ":plan.json"))
    policy = p["source_policy"]
    return {
        "frames_match_total": frames == totals["frames"],
        "ui_match_total": ui == totals["ui_submissions"],
        "submissions_match_total": frames + ui == totals["new_submissions"],
        "submissions_within_128": frames + ui <= budgets["new_submissions_max"] == 128,
        "pngs_match_total": len(names) == totals["pngs"],
        "pngs_within_20": len(names) <= budgets["pngs_max"] == 20,
        "reserves_match": (totals["reserve_submissions"] == 128 - frames - ui
                           and totals["reserve_pngs"] == 20 - len(names)),
        "png_names_unique": len(names) == len(set(names)),
        "pngs_not_more_than_frames": all(len(case["pngs"]) <= case["frames"] for case in cases.values()),
        "steps_sum_to_case_frames": all(sum(s["frames"] for s in case["steps"]) == case["frames"]
                                        for case in cases.values()),
        "case_order": list(cases) == CASE_ORDER,
        "size_within_960x540": p["review"]["size"][0] <= budgets["frame_max_width"] == 960
        and p["review"]["size"][1] <= budgets["frame_max_height"] == 540,
        "one_run_per_case": p["review"]["max_runs_per_case"] == 1,
        "builds_locked_offline_ci_j2": all(flag in p["builds"]["common_flags"]
                                           for flag in ("--locked", "--offline", "ci", "-j2"))
        and budgets["compile_jobs"] == 2,
        "review_config_equals_0072": p["review_config"] == base["review_config"],
        "current_only_is_review_with_zero_weight":
            p["current_only_config"] == {**p["review_config"], "history_weight": 0.0},
        "expected_backend_metal": p["expected_backend"] == "Metal",
        "source_policy_matches_tools": (
            set(policy["denied_source_commits"]) == set(c.DENIED_SOURCES)
            and set(policy["denied_freezes"]) == set(c.DENIED_FREEZES)
            and [tuple(s) for s in policy["allowed_feature_sets"]] == list(c.FEATURE_SETS)),
        "output_variable_allowlisted": p["review"]["output_env"] in c.OVERRIDES,
        "build_inputs_declared": p["build_inputs"] == ["crates", "Cargo.toml", "Cargo.lock", "third_party",
                                                       "tools/cargo", "rust-toolchain.toml"],
        "checks_declared_per_case": all(case["checks"] and len(set(case["checks"])) == len(case["checks"])
                                        and "frames_match_plan" in case["checks"] for case in cases.values()),
        "holder_required": "holder" in policy["required_grant_fields"],
    }


# ------------------------------------------------------------------ footprints
# Independent model of resolution::FramePlan, upscaling::Plan and metalfx::Plan.

def frame_plan(output, rect, sixteenths):
    factor = sixteenths / 16
    return [math.ceil(v * factor) for v in output], [v * factor for v in rect]


def rect_valid(size, r):
    return (all(math.isfinite(v) for v in r) and r[0] >= 0 and r[1] >= 0 and r[2] > 0 and r[3] > 0
            and r[0] + r[2] <= size[0] and r[1] + r[3] <= size[1]
            and math.ceil(r[0] + r[2] - 0.5) > math.ceil(r[0] - 0.5)
            and math.ceil(r[1] + r[3] - 0.5) > math.ceil(r[1] - 0.5))


def upscale_admits(output, rect, size, source_rect):
    if (not rect_valid(output, rect) or not rect_valid(size, source_rect)
            or size[0] > output[0] or size[1] > output[1]):
        return False
    return any([v * n / 16 for v in rect] == source_rect
               and [math.ceil(v * n / 16) for v in output] == size for n in range(4, 17))


def native_class(source_rect, rect):
    integral = lambda r: (all(math.isfinite(v) and v >= 0 and v == int(v) and v <= 8192 for v in r)
                          and r[2] != 0 and r[3] != 0)
    if not integral(source_rect) or not integral(rect):
        return "fractional"
    if source_rect[2] * rect[3] != source_rect[3] * rect[2]:
        return "aspect"
    scale = source_rect[2] / rect[2]
    if scale > 1:
        return "scale"
    return "native" if scale == 1 else "native_or_scale"


def footprint_checks(p):
    output = p["review"]["size"]
    viewports = {None: [0.0, 0.0, float(output[0]), float(output[1])],
                 "integral": p["viewports"]["integral"], "fractional": p["viewports"]["fractional"]}
    rows, ok = [], True
    for name, case in p["review"]["cases"].items():
        for index, step in enumerate(case["steps"]):
            rect = viewports[step["viewport"]]
            size, source_rect = frame_plan(output, rect, step["sixteenths"])
            row = {"case": name, "step": index, "arm": step["arm"], "sixteenths": step["sixteenths"],
                   "viewport": rect, "internal_size": size, "source_rect": source_rect,
                   "frame_plan_valid": rect_valid(output, rect)}
            if step["arm"] != "off":
                row["upscale_admitted"] = upscale_admits(output, rect, size, source_rect)
                ok = ok and row["frame_plan_valid"] and row["upscale_admitted"]
            if step["arm"] == "native":
                row["native_class"] = native_class(source_rect, rect)
                row["expected"] = step["expect"]
                ok = ok and row["native_class"] == step["expect"]
                # RequireMetalFx only where native is certain (scale 1, integral).
                ok = ok and (step["preference"] != "RequireMetalFx" or step["expect"] == "native")
            rows.append(row)
    sizes = {(r["sixteenths"], r["viewport"] == viewports[None]): r["internal_size"] for r in rows}
    return {"steps": rows, "all_classified_as_planned": ok, "internal_sizes": {
        f"{k[0]}/16{'' if k[1] else ' viewport'}": v for k, v in sorted(sizes.items())}}


# ------------------------------------------------------------------ fixture

def bodies(source):
    """Each #[test] function's body text, by name, in file order."""
    found = list(re.finditer(r"#\[test\]\s*#\[ignore[^\]]*\]\s*fn (\w+)\(\) \{\n", source))
    return {m[1]: source[m.end():(found[i + 1].start() if i + 1 < len(found) else len(source))]
            for i, m in enumerate(found)}


def config_fields(source, name):
    match = re.search(rf"const {name}: TemporalConfig = TemporalConfig \{{(.*?)\}};", source, re.S)
    return ({k: float(v) for k, v in re.findall(r"(\w+): ([0-9.]+),", match[1])}, match[1]) if match else ({}, "")


def viewport_rect(source, name):
    match = re.search(rf"const {name}: Viewport = Viewport \{{\s*rect: \[([^\]]*)\]", source)
    return [float(v) for v in match[1].split(",")] if match else None


def helper_checks(p, footprints):
    source = c.HELPER_SOURCE.read_text()
    cases = p["review"]["cases"]
    tests = bodies(source)
    opened = {m[0]: (int(m[1]), int(m[2]))
              for m in re.findall(r'Report::open\("(\w+)", (\d+), (\d+)\)', source)}
    pngs = {name: re.findall(r'report\.png\(\s*"([\w-]+\.png)"', body) for name, body in tests.items()}
    checks = {}
    for name, body in tests.items():
        start = body.find("let checks = [")
        block = body[start:body.find("];", start)] if start >= 0 else ""
        checks[name] = re.findall(r'\(\s*"(\w+)",', block)
    preferences = {name: sorted(set(re.findall(r"Preference::(\w+)", body))) for name, body in tests.items()}
    planned_preferences = {name: sorted({s["preference"] for s in case["steps"] if s["arm"] == "native"})
                           for name, case in cases.items()}
    size = re.search(r"const SIZE: \[u32; 2\] = \[(\d+), (\d+)\];", source)
    env = re.search(r'const OUTPUT_ENV: &str = "(\w+)";', source)
    review, _ = config_fields(source, "REVIEW")
    current, current_text = config_fields(source, "CURRENT_ONLY")
    sizes_used = sorted({(int(a), int(b)) for a, b in re.findall(r"internal_is\(&\w+, \[(\d+), (\d+)\]\)", source)})
    planned_sizes = {tuple(v) for v in footprints["internal_sizes"].values()}
    jitter = {"fixture": c.samples(source),
              "prepared_source_4bc810fe": c.samples(git_show(f"{PREPARED_SOURCE}:{c.JITTER_SOURCE}") or ""),
              "combined_head_0c890514": c.samples(git_show(f"{COMBINED_HEAD_AT_PREPARATION}:{c.JITTER_SOURCE}") or ""),
              "announced_535003d7": c.samples(git_show(f"{ANNOUNCED['commit']}:{c.JITTER_SOURCE}") or "")}
    canvas = re.search(r"const VIEWPORT: Viewport = Viewport \{.*?canvas_srgb: \[(\d+), (\d+), (\d+)\]", source, re.S)
    return {
        "cfg_native_metalfx_feature": source.count('#![cfg(feature = "native-metalfx")]') == 1,
        "tests_match_plan_in_order": list(tests) == list(cases),
        "limits_match_plan": all(opened.get(name) == (case["frames"], case["ui_submissions"])
                                 for name, case in cases.items()),
        "png_names_match_plan_per_case": all(sorted(pngs.get(name, [])) == sorted(case["pngs"])
                                             for name, case in cases.items()),
        "check_names_match_plan_per_case": all(checks.get(name) == case["checks"]
                                               for name, case in cases.items()),
        "preferences_match_plan_per_case": preferences == planned_preferences,
        "size_matches_plan": bool(size) and [int(size[1]), int(size[2])] == p["review"]["size"],
        "output_variable_matches_plan": bool(env) and env[1] == p["review"]["output_env"],
        "review_config_matches_plan": review == {k: float(v) for k, v in p["review_config"].items()},
        "current_only_is_zero_weight_review": current == {"history_weight": 0.0}
        and "..REVIEW" in current_text and p["current_only_config"]["history_weight"] == 0.0,
        "viewports_match_plan": viewport_rect(source, "VIEWPORT") == p["viewports"]["integral"]
        and viewport_rect(source, "FRACTIONAL_VIEWPORT") == p["viewports"]["fractional"]
        and bool(canvas) and [int(v) for v in canvas.groups()] == p["viewports"]["canvas_srgb"],
        "internal_sizes_checked_are_planned": bool(sizes_used) and set(sizes_used) <= planned_sizes,
        "jitter_copy_equals_prepared_source": jitter["fixture"] is not None
        and jitter["fixture"] == jitter["prepared_source_4bc810fe"],
        "jitter_copy_equals_announced_source": jitter["fixture"] is not None
        and jitter["fixture"] == jitter["announced_535003d7"],
        "observed": {"tests": list(tests), "limits": opened, "pngs": pngs, "checks": checks,
                     "preferences": preferences, "review_config": review,
                     "internal_sizes_checked": sizes_used, "jitter": jitter,
                     "note": "build.py re-checks the jitter table against the granted commit"},
    }


# ------------------------------------------------------------------ reuse

ITEM = re.compile(r"^(?:pub(?:\([^)]*\))? )?(const|fn|struct|impl|enum) (\w+)", re.M)


def items(source):
    """Top-level items keyed '<kind> <name>' (const and fn keyed by name), text
    from the item line to its end, excluding doc comments and attributes."""
    found = {}
    for match in ITEM.finditer(source):
        kind, name = match[1], match[2]
        start, depth, i, opened = match.start(), 0, match.start(), False
        while i < len(source):
            ch = source[i]
            if ch in "{[(":
                depth += 1
                opened = opened or ch == "{"
            elif ch in "}])":
                depth -= 1
            if depth == 0 and ((kind == "const" and ch == ";") or (kind != "const" and opened and ch == "}")):
                break
            if depth == 0 and kind in ("struct", "enum") and ch == ";":
                break
            i += 1
        key = name if kind in ("const", "fn") else f"{kind} {name}"
        found[key] = source[start:i + 1]
    return found


def reuse_checks(p):
    base_tree = p["base_fixture"]["visual_tree"]
    base = git_show(f"{base_tree}:review/temporal_visual_review.rs")
    new = c.HELPER_SOURCE.read_text()
    base_items, new_items = items(base), items(new)
    listed = p["base_fixture"]["reused_unchanged"]
    identical = {name: base_items.get(name) is not None and base_items.get(name) == new_items.get(name)
                 for name in listed}
    tools = {}
    for name, blob in p["base_fixture"]["tool_blobs"].items():
        original = git_show(f"{blob}")
        current = (c.TOOLS / name).read_text()
        diff = list(difflib.unified_diff(original.splitlines(), current.splitlines(), lineterm="", n=0))
        tools[name] = {"base_blob_present": original is not None,
                       "adapted_from_0072_noted": "handoff 0072" in current,
                       "lines_removed": sum(1 for d in diff if d.startswith("-") and not d.startswith("---")),
                       "lines_added": sum(1 for d in diff if d.startswith("+") and not d.startswith("+++"))}
    return {"base_fixture_blob_matches": c.git("rev-parse", f"{base_tree}:review/temporal_visual_review.rs").strip()
            == p["base_fixture"]["fixture_blob"],
            "reused_items_identical": all(identical.values()),
            "reused_items_checked": len(identical),
            "mismatched": sorted(name for name, same in identical.items() if not same),
            "new_or_adapted_items": sorted(set(new_items) - {k for k in new_items if base_items.get(k) == new_items[k]}),
            "tools_descend_from_0072": all(t["base_blob_present"] and t["adapted_from_0072_noted"]
                                           for t in tools.values()),
            "tools": tools}


# ------------------------------------------------------------------ grant policy

def policy_checks():
    good = {"packet": c.PACKET, "stage": c.STAGE, "source_commit": "1" * 40, "source_tree": "2" * 40,
            "visual_tree": "3" * 40, "source_freeze": {"path": "/Users/x/freeze.json", "sha256": "4" * 64},
            "features": ["native-metalfx"], "target": "/Users/andreyyurlov/.codex/metalfx-visual-target"}
    variants = {
        "explicit_uncooked_source_accepted": (good, False),
        "explicit_cooked_source_accepted": ({**good, "features": ["native-metalfx", "cooked-temporal"]}, False),
        "superseded_4bc810fe_refused": ({**good, "source_commit": "4bc810fe0daf238f219801de76b06f19e4daf826"}, True),
        "superseded_a337c86d_refused": ({**good, "source_commit": "a337c86d8d74e819b3e879d4a7db830b74de8862"}, True),
        "superseded_freeze_refused": ({**good, "source_freeze": {
            "path": "/x/freeze.json", "sha256": "b68c62d5e58b89a07aebfd17f144902917cee5890c4d09fbf06f51a64146a77b"}}, True),
        "missing_freeze_refused": ({k: v for k, v in good.items() if k != "source_freeze"}, True),
        "relative_freeze_refused": ({**good, "source_freeze": {"path": "freeze.json", "sha256": "4" * 64}}, True),
        "missing_tree_refused": ({k: v for k, v in good.items() if k != "source_tree"}, True),
        "short_commit_refused": ({**good, "source_commit": "0c890514"}, True),
        "missing_features_refused": ({k: v for k, v in good.items() if k != "features"}, True),
        "partial_cooked_features_refused": ({**good, "features": ["native-metalfx", "cooked-materials"]}, True),
        "cooked_without_metalfx_refused": ({**good, "features": ["cooked-temporal"]}, True),
        "other_packet_refused": ({**good, "packet": "0072-motion-history-numeric"}, True),
        "shared_target_refused": ({**good, "target": "/tmp/incant-cpu-profiler-target"}, True),
        "missing_target_refused": ({k: v for k, v in good.items() if k != "target"}, True),
    }
    results = {name: (c.source_problem(grant) is not None) == refused for name, (grant, refused) in variants.items()}
    return {"cases": results, "all_as_expected": all(results.values())}


# ------------------------------------------------------------------ format

def format_check():
    argv = ["rustfmt", "--edition", "2024", "--emit", "stdout"]
    env = {"HOME": os.environ.get("HOME", ""), "PATH": c.RUSTUP_BIN + ":/usr/bin:/bin",
           "RUSTUP_AUTO_INSTALL": "0"}
    source = c.HELPER_SOURCE.read_bytes()
    try:
        run = subprocess.run(argv, input=source, capture_output=True, cwd=c.ROOT, env=env, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"skipped": repr(error)}
    version = subprocess.run(["rustfmt", "--version"], capture_output=True, text=True, cwd=c.ROOT, env=env)
    return {"argv": argv, "rustfmt": version.stdout.strip(), "exit": run.returncode,
            "identical": run.returncode == 0 and run.stdout == source,
            "method": "stdin to stdout and byte comparison (stdin --check exits 0 even with a diff)"}


# ------------------------------------------------------------------ announced source

def announced_source_checks(p):
    """build.py's freeze rule applied read-only to root's announced commit: git
    objects stand in for the exported files (no export, target or build)."""
    freeze_path = Path(ANNOUNCED["freeze"])
    result = {"announced": ANNOUNCED}
    if not freeze_path.is_file():
        return {**result, "problem": "the announced freeze is absent", "all_pass": False}
    result["freeze_sha256_matches"] = c.sha(freeze_path) == ANNOUNCED["freeze_sha256"]
    rows, problem = build.freeze_rows(c.read_json(freeze_path), ANNOUNCED["commit"])
    result["freeze_shape_accepted"] = problem is None
    if problem:
        return {**result, "problem": problem, "all_pass": False}
    result["tree_matches"] = c.git("rev-parse", ANNOUNCED["commit"] + "^{tree}").strip() == ANNOUNCED["tree"]
    listing = subprocess.run(["git", "ls-tree", "-r", "-l", "-z", "--full-tree", ANNOUNCED["commit"]],
                             cwd=c.ROOT, capture_output=True, check=True).stdout
    blobs = {}
    for record in listing.split(b"\0"):
        if record:
            meta, path = record.split(b"\t", 1)
            _, kind, blob, size = meta.decode().split()
            if kind == "blob":
                blobs[path.decode()] = (blob, int(size))
    roots = p["build_inputs"]
    inputs = {path for path in blobs if build.under(path, roots)}
    frozen = {path for path in rows if build.under(path, roots)}
    digests = {}
    reader = subprocess.Popen(["git", "cat-file", "--batch"], cwd=c.ROOT, stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE)
    try:
        for path in sorted(set(rows) & set(blobs)):
            reader.stdin.write(blobs[path][0].encode() + b"\n")
            reader.stdin.flush()
            size = int(reader.stdout.readline().split()[2])
            digests[path] = hashlib.sha256(reader.stdout.read(size)).hexdigest()
            reader.stdout.read(1)
    finally:
        reader.stdin.close()
        reader.wait()
    mismatched = sorted(path for path, digest in digests.items()
                        if digest != rows[path]["sha256"]
                        or rows[path]["bytes"] not in (None, blobs[path][1]))
    result.update({
        "build_inputs": len(inputs), "freeze_rows": len(rows), "freeze_build_input_rows": len(frozen),
        "freeze_rows_outside_roots": sorted(set(rows) - frozen),
        "inputs_missing_from_freeze": sorted(inputs - frozen)[:20],
        "freeze_rows_missing_from_commit": sorted(set(rows) - set(blobs))[:20],
        "blob_sha256_compared": len(digests), "mismatched": mismatched[:20],
        "exact": bool(inputs) and inputs == frozen and set(rows) <= set(blobs) and not mismatched})
    result["all_pass"] = (result["freeze_sha256_matches"] and result["freeze_shape_accepted"]
                          and result["tree_matches"] and result["exact"])
    return result


# ------------------------------------------------------------------ refusals

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
    footprints = footprint_checks(p)
    result = {"python": sys.version.split()[0], "plan_sha256": c.sha(c.PLAN_PATH),
              "helper_sha256": c.sha(c.HELPER_SOURCE),
              "tools": {path.name: c.sha(path) for path in sorted(c.TOOLS.glob("*.py"))},
              "syntax": "all tools parse", "plan": plan_checks(p), "footprints": footprints,
              "helper": helper_checks(p, footprints), "reuse": reuse_checks(p),
              "grant_policy": policy_checks(), "format": format_check(),
              "announced_source": announced_source_checks(p), "refusals": refusal_checks()}
    refusals = result["refusals"]
    result["all_pass"] = (all(result["plan"].values())
                          and footprints["all_classified_as_planned"]
                          and all(v for k, v in result["helper"].items() if k != "observed")
                          and result["reuse"]["base_fixture_blob_matches"]
                          and result["reuse"]["reused_items_identical"]
                          and result["reuse"]["tools_descend_from_0072"]
                          and result["grant_policy"]["all_as_expected"]
                          and result["format"].get("identical", False)
                          and result["announced_source"]["all_pass"]
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
