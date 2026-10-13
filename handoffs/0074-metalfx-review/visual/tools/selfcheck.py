#!/usr/bin/env python3
"""Preparation self-check for handoff 0074 stage B. Standard library only; no
grant needed. Never compiles, starts a renderer, acquires a target or touches
the stage evidence directory or the global GPU lock.

Adapted from handoff 0072's committed visual/tools/selfcheck.py (visual tree
ac8a48de).

  python3 -I -B selfcheck.py [--write <new-output.json>]

The active plan is the metalfx-late stage's late-plan.json; plan.json is the
frozen 002 stage it extends.

1. Every stage Python tool parses (ast only; no bytecode is written).
2. late-plan.json is internally consistent, within the brief's 160-submission,
   16-PNG and 120 s budgets, and keeps 0072's review policy unchanged.
3. An independent model of the frame, upscaling and corrected native plans
   (output/input factors clamped to [1, 4], on root's measured device range)
   classifies every planned step exactly as the plan expects (native or
   FractionalViewport) and admits every portable footprint.
4. The Rust fixture agrees with the plan: the frozen five tests then the late
   tests in order, the listing filter selecting exactly the late tests,
   Report::open limits, PNG names and required check names per case, frame size,
   output variable, configs, viewports, preferences, planned internal sizes and
   the jitter table of the source.
5. Every item the plan lists as reused is byte-identical to 0072's reviewed
   fixture blob; the adapted tools descend from 0072's reviewed tool blobs.
6. The grant policy refuses absent, superseded or ill-formed sources (pure).
7. rustfmt (edition 2024, stdin to stdout, no Cargo or target) leaves the
   fixture byte-identical.
8. Root's announced source (read-only git objects, no export): the commit has
   its tree, descends from the reviewed 535003d7 and changes only the native
   scale-unit build inputs, and the fixture's jitter copy equals its table. An
   announced freeze must have its SHA-256, the {commit, files} shape build.py
   accepts and exact build-input coverage by path, size and blob SHA-256;
   until root announces one, build.py checkout enforces that at grant time.
9. With the grant absent, every guarded command refuses with the grant message,
   creates no evidence directory, leaves `git status` unchanged and does not
   touch the global native-gpu.lock.
10. The integral 12/16 and 8/16 native steps cannot pass through a portable
   fallback: plan and fixture bind them to MetalFx alone; the superseded
   535003d7 unit model reproduces the first execution's recorded selections,
   whose Scale fallbacks the current expectations refuse; and the first
   execution's fixture fails the same fixture check (negative control). This
   runs on the frozen 002 plan, then on the late cases' bindings.
11. The prior stage stays immutable and reusable: plan.json is the 002 blob,
   the fixture starts with the 002 fixture byte for byte and appends only the
   late tests, and the late plan's expected references and striation rectangle
   match the committed 002 records.
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
            ["gpu.py", "list", "review"], ["gpu.py", "review", "late_stationary"],
            ["gpu.py", "review", "late_motion"], ["gpu.py", "release"],
            ["analyze.py", "check"], ["analyze.py", "publish"]]
CASE_ORDER = ["late_stationary", "late_motion"]
# The brief's late-stage caps: world submissions, PNGs, seconds per case.
CAPS = {"submissions": 160, "pngs": 16, "case_timeout_s": 120}
# The frozen 002 stage the late stage extends without changing it.
PRIOR_PLAN = c.VISUAL / "plan.json"
PRIOR_CASES = ["stationary", "motion", "excluded", "transitions", "viewport"]
SECOND_TRANSITIONS = ("handoffs/0074-metalfx-review/visual-evidence-002/reports/transitions.json",
                      "150e76cb02e37ed463ddab39cddf09f739426fd980d4ce75b509319e86762aab")
PREPARED_SOURCE = "4bc810fe0daf238f219801de76b06f19e4daf826"
COMBINED_HEAD_AT_PREPARATION = "0c890514f84c09cb98358ba6d7fa9431140b5b7c"
# Announced in root's scale-correction revision and frozen by root for the 002
# build; the late stage reuses the same source and freeze. The grant comes later.
ANNOUNCED = {"commit": "077f7c39c0f4122aae36ea3a88537be006722822",
             "tree": "408c4bb2948983cfde980fa40c31f443b5d78a0c",
             "reviewed": "535003d766bcc1386fd2d2f454c83808a0fcc4c8",
             "changed_inputs": ["crates/incant_render/src/metalfx/native.rs",
                                "crates/incant_render/src/metalfx/numerics.rs",
                                "crates/incant_render/src/metalfx/plan.rs"],
             "freeze": "/Users/andreyyurlov/.codex/incant-evidence/metalfx-scale-correction-001/committed-build-source.json",
             "freeze_sha256": "62db43cb2b769b89fde5431c776fc6d89ca670d67ae87ff7317ac1e75f2f33a9"}
# Root's capability query on this device: an API query with 0 submissions.
SCALE_QUERY = {"path": "/Users/andreyyurlov/.codex/incant-evidence/metalfx-scale-query-001/receipt.json",
               "sha256": "e5610fe06e38b64f46186cb5ca71d8ac555a3183a4dfbe0e8b71b081d653abc9"}
# The first execution (2dc2db04): its granted fixture, and its recorded reports
# (SHA-256 as in visual-evidence/manifest.json) with their selection groups in
# the order of the plan's native steps.
FIRST_FIXTURE = "b2c3e14e61ddfb2980ba3063a7148b424fb0d27c:review/metalfx_visual_review.rs"
FIRST_REPORTS = {
    "transitions": ("7bcc167971b8ec016cc3077cbdcc7e7e89e88d97582f9b0c85d099a1af930d58",
                    ["view_a", "cut", "d12", "d8", "d10", "resumed"]),
    "viewport": ("3cf84253ee4e7122648a0ea91133b46721a463b1e7c4cfe77fea7e348f5cd04f",
                 ["framed", "framed_8_16", "fractional"]),
}
SELECTION = {"native": "MetalFx", "scale": "Portable(Some(Scale))", "aspect": "Portable(Some(Aspect))",
             "fractional": "Portable(Some(FractionalViewport))"}
# What the fixture's Expect::allows (checked verbatim against EXPECT_IMPL) accepts.
ALLOWS = {"native": {SELECTION["native"]}, "fractional": {SELECTION["fractional"]}}
EXPECT_IMPL = """impl Expect {
    fn allows(self, selection: Option<Selection>) -> bool {
        match self {
            Self::Native => selection == Some(Selection::MetalFx),
            Self::Fractional => {
                selection == Some(Selection::Portable(Some(Fallback::FractionalViewport)))
            }
        }
    }
}"""


def git_show(spec):
    run = subprocess.run(["git", "show", spec], cwd=c.ROOT, capture_output=True, text=True)
    return run.stdout if run.returncode == 0 else None


# ------------------------------------------------------------------ plan

def plan_checks(p, prior):
    cases = p["review"]["cases"]
    budgets, totals = p["budgets"], p["totals"]
    frames = sum(case["frames"] for case in cases.values())
    ui = sum(case["ui_submissions"] for case in cases.values())
    names = [name for case in cases.values() for name in case["pngs"]]
    base = json.loads(git_show(prior["base_fixture"]["visual_tree"] + ":plan.json"))
    policy = p["source_policy"]
    return {
        "frames_match_total": frames == totals["frames"],
        "ui_match_total": ui == totals["ui_submissions"],
        "submissions_match_total": frames + ui == totals["new_submissions"],
        "submissions_within_cap": frames + ui <= budgets["new_submissions_max"] == CAPS["submissions"],
        "pngs_match_total": len(names) == totals["pngs"],
        "pngs_within_cap": len(names) <= budgets["pngs_max"] == CAPS["pngs"],
        "case_timeout_within_cap": budgets["review_case_timeout_s"] <= CAPS["case_timeout_s"],
        "reserves_match": (totals["reserve_submissions"] == CAPS["submissions"] - frames - ui
                           and totals["reserve_pngs"] == CAPS["pngs"] - len(names)),
        "retained_frames_match_pngs": all(
            sum(len(s.get("retained_frames", [])) for s in case["steps"]) == len(case["pngs"])
            and all(1 <= k <= s["frames"] for s in case["steps"] for k in s.get("retained_frames", []))
            for case in cases.values()),
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


def measured_range():
    """Root's pinned raw [min, max] for this device, or None if it does not verify."""
    path = Path(SCALE_QUERY["path"])
    if not path.is_file() or c.sha(path) != SCALE_QUERY["sha256"]:
        return None
    query = c.read_json(path)["query"]
    if not (query.get("supported") is True and query.get("gpu_submissions") == 0):
        return None
    return [float(query["raw_min_scale"]), float(query["raw_max_scale"])]


def native_class(source_rect, rect, raw, superseded=False):
    """metalfx::plan::Plan::new on the device's raw range. Corrected (077f7c39):
    raw output/input factors clamped to [1, 4] bound output/source. Superseded
    (535003d7): the same values read as content fractions in [0.25, 1] bound
    source/output, so a [1, 3] device admitted only content scale 1."""
    integral = lambda r: (all(math.isfinite(v) and v >= 0 and v == int(v) and v <= 8192 for v in r)
                          and r[2] != 0 and r[3] != 0)
    if not integral(source_rect) or not integral(rect):
        return "fractional"
    if source_rect[2] * rect[3] != source_rect[3] * rect[2]:
        return "aspect"
    if raw is None:
        return "unmeasured"
    if not all(math.isfinite(v) and v > 0 for v in raw) or raw[0] > raw[1]:
        return "scale"
    if superseded:
        low, high, value = max(raw[0], 0.25), min(raw[1], 1.0), source_rect[2] / rect[2]
    else:
        low, high, value = max(raw[0], 1.0), min(raw[1], 4.0), rect[2] / source_rect[2]
    return "native" if low <= value <= high else "scale"


def footprint_checks(p, raw):
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
                row["native_class"] = native_class(source_rect, rect, raw)
                row["expected"] = step["expect"]
                ok = ok and row["native_class"] == step["expect"]
                # RequireMetalFx only on steps that must select MetalFX.
                ok = ok and (step["preference"] != "RequireMetalFx" or step["expect"] == "native")
            rows.append(row)
    sizes = {(r["sixteenths"], r["viewport"] == viewports[None]): r["internal_size"] for r in rows}
    return {"device_raw_range": raw, "steps": rows, "all_classified_as_planned": ok, "internal_sizes": {
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
    planned_bodies = "".join(tests.get(name, "") for name in cases)
    sizes_used = sorted({(int(a), int(b))
                         for a, b in re.findall(r"internal_is\(&\w+, \[(\d+), (\d+)\]\)", planned_bodies)})
    # libtest lists a test when its name contains any filter; one filter is planned.
    filters = p["review"].get("list_args", [])[2:]
    listed = [name for name in tests if any(f in name for f in filters)] if filters else list(tests)
    planned_sizes = {tuple(v) for v in footprints["internal_sizes"].values()}
    jitter = {"fixture": c.samples(source),
              "prepared_source_4bc810fe": c.samples(git_show(f"{PREPARED_SOURCE}:{c.JITTER_SOURCE}") or ""),
              "combined_head_0c890514": c.samples(git_show(f"{COMBINED_HEAD_AT_PREPARATION}:{c.JITTER_SOURCE}") or ""),
              "announced_077f7c39": c.samples(git_show(f"{ANNOUNCED['commit']}:{c.JITTER_SOURCE}") or "")}
    canvas = re.search(r"const VIEWPORT: Viewport = Viewport \{.*?canvas_srgb: \[(\d+), (\d+), (\d+)\]", source, re.S)
    return {
        "cfg_native_metalfx_feature": source.count('#![cfg(feature = "native-metalfx")]') == 1,
        "tests_match_plan_in_order": list(tests) == PRIOR_CASES + list(cases),
        "list_filter_selects_plan_cases": p["review"].get("list_args", [])[:2] == ["--list", "--ignored"]
        and len(filters) == 1 and listed == list(cases),
        "limits_match_plan": all(opened.get(name) == (case["frames"], case["ui_submissions"])
                                 for name, case in cases.items()),
        "png_names_match_plan_per_case": all(sorted(pngs.get(name, [])) == sorted(case["pngs"])
                                             for name, case in cases.items()),
        "check_names_match_plan_per_case": all(checks.get(name) == case["checks"]
                                               for name, case in cases.items()),
        "preferences_match_plan_per_case": {name: preferences.get(name) for name in cases} == planned_preferences,
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
        and jitter["fixture"] == jitter["announced_077f7c39"],
        "observed": {"tests": list(tests), "listed_by_filter": listed, "limits": opened, "pngs": pngs, "checks": checks,
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
        "announced_corrected_source_accepted": ({**good, "source_commit": ANNOUNCED["commit"],
                                                 "source_tree": ANNOUNCED["tree"]}, False),
        "core_presentation_target_accepted": (
            {**good, "target": "/Users/andreyyurlov/.codex/incant-core-presentation-target"}, False),
        "superseded_4bc810fe_refused": ({**good, "source_commit": "4bc810fe0daf238f219801de76b06f19e4daf826"}, True),
        "superseded_a337c86d_refused": ({**good, "source_commit": "a337c86d8d74e819b3e879d4a7db830b74de8862"}, True),
        "superseded_535003d7_refused": ({**good, "source_commit": ANNOUNCED["reviewed"]}, True),
        "superseded_freeze_refused": ({**good, "source_freeze": {
            "path": "/x/freeze.json", "sha256": "b68c62d5e58b89a07aebfd17f144902917cee5890c4d09fbf06f51a64146a77b"}}, True),
        "superseded_535003d7_freeze_refused": ({**good, "source_freeze": {
            "path": "/x/freeze.json", "sha256": "bd8295b0eb3174b96eb5ff73b0e0e7c464c6c5d620fd66df37bd59d3eafe56a0"}}, True),
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
    """Root's announced commit, read-only from git objects (no export, target or
    build): its tree, its descent from the reviewed source and the build inputs
    it changes. Once root announces a freeze, build.py's freeze rule is applied
    the same way, git objects standing in for the exported files."""
    commit, roots = ANNOUNCED["commit"], p["build_inputs"]
    descends = subprocess.run(["git", "merge-base", "--is-ancestor", ANNOUNCED["reviewed"], commit],
                              cwd=c.ROOT, capture_output=True).returncode == 0
    changed = c.git("diff", "--name-only", "--no-renames", ANNOUNCED["reviewed"], commit).splitlines()
    result = {"announced": ANNOUNCED,
              "tree_matches": c.git("rev-parse", commit + "^{tree}").strip() == ANNOUNCED["tree"],
              "descends_from_reviewed_source": descends,
              "build_inputs_changed": sorted(path for path in changed if build.under(path, roots))}
    result["only_scale_unit_inputs_changed"] = result["build_inputs_changed"] == ANNOUNCED["changed_inputs"]
    listing = subprocess.run(["git", "ls-tree", "-r", "-l", "-z", "--full-tree", commit],
                             cwd=c.ROOT, capture_output=True, check=True).stdout
    blobs = {}
    for record in listing.split(b"\0"):
        if record:
            meta, path = record.split(b"\t", 1)
            _, kind, blob, size = meta.decode().split()
            if kind == "blob":
                blobs[path.decode()] = (blob, int(size))
    inputs = {path for path in blobs if build.under(path, roots)}
    result["build_inputs"] = len(inputs)
    scope = result["tree_matches"] and descends and result["only_scale_unit_inputs_changed"]
    if ANNOUNCED["freeze"] is None:
        return {**result, "freeze": ("not announced; build.py checkout refuses an ill-formed freeze before "
                                     "export and latches stop unless the exported build inputs match it "
                                     "exactly"), "all_pass": scope}
    freeze_path = Path(ANNOUNCED["freeze"])
    if not freeze_path.is_file():
        return {**result, "problem": "the announced freeze is absent", "all_pass": False}
    result["freeze_sha256_matches"] = c.sha(freeze_path) == ANNOUNCED["freeze_sha256"]
    rows, problem = build.freeze_rows(c.read_json(freeze_path), commit)
    result["freeze_shape_accepted"] = problem is None
    if problem:
        return {**result, "problem": problem, "all_pass": False}
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
        "freeze_rows": len(rows), "freeze_build_input_rows": len(frozen),
        "freeze_rows_outside_roots": sorted(set(rows) - frozen),
        "inputs_missing_from_freeze": sorted(inputs - frozen)[:20],
        "freeze_rows_missing_from_commit": sorted(set(rows) - set(blobs))[:20],
        "blob_sha256_compared": len(digests), "mismatched": mismatched[:20],
        "exact": bool(inputs) and inputs == frozen and set(rows) <= set(blobs) and not mismatched})
    result["all_pass"] = scope and result["freeze_sha256_matches"] and result["exact"]
    return result


# ------------------------------------------------------------------ strict native steps

def expectation_checks(source, p, names=tuple(FIRST_REPORTS)):
    """The fixture binds the plan's native steps to Selection::MetalFx alone."""
    enum = re.search(r"\nenum Expect \{\n(.*?)\n\}", source, re.S)
    impl = re.search(r"\nimpl Expect \{\n.*?\n\}\n", source, re.S)
    tests, bound = bodies(source), {}
    for name in names:
        body = tests.get(name, "")
        start = body.find("let steps = [")
        block = body[start:body.find("];", start)] if start >= 0 else ""
        bound[name] = [e.lower() for e in re.findall(r"\(Expect::(\w+), \w+\.as_slice\(\)\)", block)]
    planned = {name: [s["expect"] for s in p["review"]["cases"][name]["steps"] if s["arm"] == "native"]
               for name in names}
    return {
        "variants_are_native_and_fractional": bool(enum)
        and re.findall(r"^    (\w+),$", enum[1], re.M) == ["Native", "Fractional"],
        "native_allows_only_metalfx": bool(impl) and impl[0].split() == EXPECT_IMPL.split(),
        "no_scale_fallback_accepted": "NativeOrScale" not in source and "Fallback::Scale" not in source,
        "steps_bound_as_planned": all(bound[n] and bound[n] == planned[n] for n in bound),
        "selections_checked_against_steps": all(
            '("selections_as_planned", steps_as_planned(&steps))' in tests.get(n, "") for n in bound),
    }


def strict_native_checks(p, footprints):
    """The 12/16 and 8/16 native steps cannot pass through a portable fallback.
    The first execution's recorded selections are replayed against both unit
    models and against the current expectations (ALLOWS mirrors Expect::allows)."""
    raw, cases = footprints["device_raw_range"], p["review"]["cases"]
    reduced = [s for case in cases.values() for s in case["steps"]
               if s["arm"] == "native" and s["sixteenths"] in (12, 8)]
    fixture = expectation_checks(c.HELPER_SOURCE.read_text(), p)
    first_fixture = expectation_checks(git_show(FIRST_FIXTURE) or "", p)
    replay, recorded_ok = [], True
    for case, (digest, groups) in FIRST_REPORTS.items():
        path = c.PACKET_DIR / "visual-evidence" / "reports" / f"{case}.json"
        if not path.is_file() or c.sha(path) != digest:
            recorded_ok = False
            continue
        report = c.read_json(path)
        recorded_ok = recorded_ok and report["checks"].get("selections_as_planned") is True
        rows = [r for r in footprints["steps"] if r["case"] == case and r["arm"] == "native"]
        steps = [s for s in cases[case]["steps"] if s["arm"] == "native"]
        if not len(groups) == len(rows) == len(steps):
            recorded_ok = False
            continue
        for group, row, step in zip(groups, rows, steps):
            shots = report["selections"][group]
            recorded = sorted({s["metalfx_selection"] for s in shots})
            replay.append({
                "case": case, "group": group, "sixteenths": step["sixteenths"], "recorded": recorded,
                "frames_and_size_as_planned": len(shots) == step["frames"] and all(
                    s["upscale"]["internal_size"] == row["internal_size"] for s in shots),
                "superseded_model": SELECTION.get(native_class(row["source_rect"], row["viewport"], raw, True)),
                "corrected_model": SELECTION.get(row["native_class"]),
                "expect": step["expect"], "accepted_now": set(recorded) <= ALLOWS.get(step["expect"], set())})
    lowered = [r for r in replay if r["sixteenths"] in (12, 8)]
    return {
        "measured_raw_range": raw,
        "reduced_steps_expect_native": len(reduced) == 3 and all(s["expect"] == "native" for s in reduced),
        "expectations_are_native_or_fractional": {s.get("expect") for case in cases.values()
                                                  for s in case["steps"]} <= {None, "native", "fractional"},
        "fixture": fixture,
        "fixture_binds_metalfx": all(fixture.values()),
        "first_fixture": first_fixture,
        # Each detector alone refuses the first fixture, which accepted Scale.
        "first_fixture_refused": not any(first_fixture[k] for k in (
            "variants_are_native_and_fractional", "native_allows_only_metalfx",
            "no_scale_fallback_accepted", "steps_bound_as_planned")),
        "replay": replay,
        "first_records_verified_and_passed_then": recorded_ok and len(replay) == 9,
        "replay_frames_and_sizes_as_planned": bool(replay) and all(r["frames_and_size_as_planned"] for r in replay),
        "superseded_model_reproduces_records": bool(replay)
        and all(r["recorded"] == [r["superseded_model"]] for r in replay),
        "corrected_model_selects_metalfx_at_reduced_steps": len(lowered) == 3
        and all(r["corrected_model"] == SELECTION["native"] for r in lowered),
        "recorded_scale_fallbacks_now_refused": len(lowered) == 3
        and all(r["recorded"] == [SELECTION["scale"]] and not r["accepted_now"] for r in lowered),
        "other_recorded_steps_still_accepted": all(r["accepted_now"] for r in replay if r not in lowered),
    }


# ------------------------------------------------------------------ late stage

def late_checks(p, prior):
    """The late stage extends the frozen 002 stage without changing it: plan.json
    is still the 002 blob, the fixture starts with the 002 fixture byte for byte
    and appends exactly the late tests, every late native step binds MetalFx
    alone, and the plan's 002 references and striation rectangle match the
    committed 002 records."""
    stage = p["prior_stage"]
    tree = stage["visual_tree"]
    run = subprocess.run(["git", "show", f"{tree}:review/metalfx_visual_review.rs"], cwd=c.ROOT,
                         capture_output=True)
    frozen = run.stdout if run.returncode == 0 else b""
    source = c.HELPER_SOURCE.read_bytes()
    appended = source[len(frozen):].decode()
    cases = list(p["review"]["cases"])
    binding = expectation_checks(source.decode(), p, cases)
    record = c.ROOT / SECOND_TRANSITIONS[0]
    rgba = ({f["name"]: f["rgba_sha256"] for f in c.read_json(record)["frames"]}
            if record.is_file() and c.sha(record) == SECOND_TRANSITIONS[1] else {})
    expected = p["review"]["cases"]["late_stationary"]["expected_reference_rgba_sha256"]
    rect = re.search(r"const STRIATION_RECT: \[usize; 4\] = \[(\d+), (\d+), (\d+), (\d+)\];", appended)
    return {
        "prior_plan_is_the_002_blob": prior["stage"] == "metalfx-visual"
        and c.git("rev-parse", f"{tree}:plan.json").strip() == stage["plan_blob"] == c.git_blob_sha1(PRIOR_PLAN),
        "prior_fixture_is_an_exact_prefix": bool(frozen) and source.startswith(frozen)
        and len(frozen) == stage["fixture_bytes"]
        and hashlib.sha256(frozen).hexdigest() == stage["fixture_sha256"]
        and c.git("rev-parse", f"{tree}:review/metalfx_visual_review.rs").strip() == stage["fixture_blob"],
        "appended_section_adds_exactly_the_late_tests": list(bodies(appended)) == cases,
        "prior_cases_are_the_002_plan_cases": stage["cases"] == list(prior["review"]["cases"]) == PRIOR_CASES,
        "late_native_steps_bind_metalfx": all(binding.values()),
        "expected_references_match_002_records": bool(rgba) and expected["off"] == rgba.get("off-b")
        and expected["off-d8"] == rgba.get("off-b-d8"),
        "striation_rect_matches_002_measurement": bool(rect)
        and [int(v) for v in rect.groups()] == [422, 330, 462, 380],
        "observed_binding": binding,
    }


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
    p, prior, raw = c.plan(), c.read_json(PRIOR_PLAN), measured_range()
    footprints, prior_footprints = footprint_checks(p, raw), footprint_checks(prior, raw)
    result = {"python": sys.version.split()[0], "plan_sha256": c.sha(c.PLAN_PATH),
              "prior_plan_sha256": c.sha(PRIOR_PLAN), "helper_sha256": c.sha(c.HELPER_SOURCE),
              "tools": {path.name: c.sha(path) for path in sorted(c.TOOLS.glob("*.py"))},
              "syntax": "all tools parse", "plan": plan_checks(p, prior), "footprints": footprints,
              "helper": helper_checks(p, footprints), "reuse": reuse_checks(prior),
              "grant_policy": policy_checks(), "format": format_check(),
              "announced_source": announced_source_checks(p),
              "prior_strict_native": strict_native_checks(prior, prior_footprints),
              "late": late_checks(p, prior), "refusals": refusal_checks()}
    refusals = result["refusals"]
    strict = {k: v for k, v in result["prior_strict_native"].items()
              if k not in ("measured_raw_range", "fixture", "first_fixture", "replay")}
    late = {k: v for k, v in result["late"].items() if not k.startswith("observed")}
    result["all_pass"] = (all(result["plan"].values())
                          and footprints["all_classified_as_planned"]
                          and prior_footprints["all_classified_as_planned"]
                          and all(late.values())
                          and all(v for k, v in result["helper"].items() if k != "observed")
                          and result["reuse"]["base_fixture_blob_matches"]
                          and result["reuse"]["reused_items_identical"]
                          and result["reuse"]["tools_descend_from_0072"]
                          and result["grant_policy"]["all_as_expected"]
                          and result["format"].get("identical", False)
                          and result["announced_source"]["all_pass"]
                          and all(strict.values())
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
