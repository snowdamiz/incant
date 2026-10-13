#!/usr/bin/env python3
"""Compose the filter execution plan (ssao-filter-002/plan.json). CPU only.

The same bounded sequence as 0070-ssao-filter-001, repeated once in a new
namespace so one report stays tied to one frozen build: the numeric cases
come from ssao-repair/filter/cases.json; the appearance set is build004's,
unchanged; the two control tests and their submission counts are root's
(9f3a6b03). The build adds the helper's CPU guard regression. The engine
commit stays pending until root picks the final candidate; root supplies it
at the source freeze and it is verified against HEAD's engine source.

  python3 -I -B make_plan.py                      # keep the plan's engine_commit
  python3 -I -B make_plan.py --engine-commit SHA  # bind the frozen engine source
  python3 -I -B make_plan.py --check              # plan.json is exactly reproducible
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "tools")]
import filter_common as fc  # noqa: E402

CASES = json.loads((fc.PACKET_DIR / "ssao-repair" / "filter" / "cases.json").read_text())
BUILD004_PLAN = json.loads((fc.BASE_TOOLS / "plan.json").read_text())
NEW_FRAMES_MAX = 128

CONTROLS = [
    {"name": "filter_device", "binary": "lib",
     "test": "ambient_occlusion::filter_device_tests::actual_plane_filter_horizontal_vertical_depth_and_r8_contract",
     "source": "crates/incant_render/src/ambient_occlusion/filter_device_tests.rs",
     "frames": 2, "render_passes": 6, "images": 0, "required_before_cases": True,
     "counted": "two 19x15 submissions, six render passes, no images (root, 9f3a6b03)"},
    {"name": "ao_device", "binary": "lib",
     "test": "ambient_occlusion::device_tests::"
             "explicit_opt_in_zero_strength_resize_queued_frames_and_failure_preserve_default_output",
     "source": "crates/incant_render/src/ambient_occlusion/device_tests.rs",
     "frames": 7, "images": 0, "required_before_cases": True,
     "counted": "3 sizes x (reference + AO frame) + empty scene"},
    {"name": "ao_device_cooked", "binary": "lib-cooked",
     "test": "ambient_occlusion::device_tests::"
             "explicit_opt_in_zero_strength_resize_queued_frames_and_failure_preserve_default_output",
     "source": "crates/incant_render/src/ambient_occlusion/device_tests.rs",
     "frames": 0, "images": 0, "required_before_cases": False, "optional": True,
     "counted": "typed CookedMaterials refusal before any submission (cfg cooked-materials branch)"},
]
ROUTES = {case["name"]: {"renderers": case["renderers"], "route": case["route"], "covers": case["covers"]}
          for case in BUILD004_PLAN["numeric_cases"] if case["name"] != "bloom_numeric"}
ROUTES["ssao_quality"] = {
    "renderers": ["off", "ao"],
    "route": "SceneInput + Renderer::prepare + with_camera + draw_scene, 321x193 Rgba8UnormSrgb, offset fractional viewport",
    "covers": ["isolated gltf::sphere(24, 48) x 1.2 at view (0, 0, -3), gray 0.5: AO identity in both projections",
               "N8 stripes: four 0.1-wide quads, pitch 0.3, y -0.5..1.5, z = -3, before the z = -4 receiver",
               "white on/off and black basis per projection; stage planes for both AO-on stripe frames"]}
FEATURES = {
    "lib_requested": ["ssao-stage-readback"],
    "lib_expected": ["default", "physics-deterministic", "ssao-stage-readback"],
    "helper_incant_render_expected": ["default", "physics-deterministic", "ssao-stage-readback"],
    "lib_cooked_requested": ["cooked-materials", "ssao-stage-readback"],
    "lib_cooked_required": ["cooked-materials", "ssao-stage-readback"],
    "note": ("Test default shader features include the non-default diagnostic ssao-stage-readback; "
             "the build records the actual compiler-artifact features and refuses any other set."),
}
ANALYZER_REL = f"{fc.REPAIR_REL}/filter/analyze_filter.py"
ANALYZE = f"{fc.NAMESPACE_REL}/analyze.py"
RUNNER = f"python3 -I -B {fc.NAMESPACE_REL}/filter_runner.py"
GUARD = {
    "build_child": "build-helper-guard-tests",
    "command": "cargo test --manifest-path <helper>/Cargo.toml --lib -- sphere_guard (CPU only)",
    "tests": [f"numeric::sphere_guard_tests::{n}" for n in (
        "sphere_guard_fractional_rim_alone_is_not_sphere",
        "sphere_guard_interior_disc_counts_within_tolerance",
        "sphere_guard_tolerance_stays_ten_percent")],
    "asserts": ["the fractional viewport rim alone (601 canvas-blend texels) counts as no sphere",
                "an interior disc of the analytic area counts exactly its texels in both projections",
                "the coverage tolerance stays 10% (3419 fails, 2818 passes for perspective)"],
    "rule": "the build records nothing unless exactly these three tests run and pass",
}


def numeric_cases():
    out = []
    for case in CASES["numeric_cases"]:
        out.append({"name": case["name"], "frames": case["frames"], "stage_frames": case["stage_frames"],
                    **ROUTES[case["name"]]})
    return out


def verify_engine(commit):
    """The frozen engine commit exists and carries both control tests and the
    ssao-stage-readback feature; HEAD's engine inputs are checked at build time."""
    fc.git("cat-file", "-e", f"{commit}^{{commit}}")
    for ctl in CONTROLS:
        text = fc.git("show", f"{commit}:{ctl['source']}")
        name = ctl["test"].rsplit("::", 1)[1]
        if f"fn {name}()" not in text:
            raise SystemExit(f"{ctl['source']} at {commit} has no fn {name}()")
    manifest = fc.git("show", f"{commit}:crates/incant_render/Cargo.toml")
    if "ssao-stage-readback" not in manifest:
        raise SystemExit(f"crates/incant_render/Cargo.toml at {commit} declares no ssao-stage-readback feature")


def build(engine_commit):
    numeric = numeric_cases()
    controls_frames = sum(c["frames"] for c in CONTROLS)
    numeric_frames = sum(c["frames"] for c in numeric)
    appearance = BUILD004_PLAN["appearance"]
    planned = controls_frames + numeric_frames + appearance["renders"]
    children = len(CONTROLS) + len(numeric) + 1
    repo_config = fc.ROOT / ".cargo" / "config.toml"
    return {
        "format": "incant-0070-ssao-filter-plan-v1",
        "packet": fc.PACKET,
        "namespace": fc.NAMESPACE,
        "engine_commit": engine_commit,
        "source_binding": {
            "engine_candidate": ("pending root's final pick: R1/R3 as measured (5dfc522ec164f2bc50d760f12db57eae233f33d8) "
                                 "or a repaired sampling candidate (next-repair-preparation.md)"),
            "freeze": ("root commits the chosen engine on this branch, then runs make_plan.py "
                       "--engine-commit <that commit>; the source freeze requires HEAD's engine inputs "
                       "to equal it byte for byte"),
            "oracle_condition": ("analyze_filter.py's oracle and quality model encode R1/R3's rotation and "
                                 "phase; a different sampling candidate needs the matching oracle change "
                                 "before the freeze, or its numeric gates fail by construction"),
            "previous_namespace": "0070-ssao-filter-001 stays latched on case-ssao_quality; nothing of it is reused",
        },
        "guard_regression": GUARD,
        "tool_revision": fc.tool_revision(),
        "features": FEATURES,
        "frame_rule": BUILD004_PLAN["frame_rule"],
        "controls": CONTROLS,
        "numeric_cases": numeric,
        "not_run": {"bloom_numeric": "bloom code unchanged by the filter; the DRS bloom centroid stays in lifecycle_drs",
                    "full_lib_gpu_suite": "not repeated; prior qualified unaffected areas remain evidence"},
        "appearance": appearance,
        "analysis": {
            "after": "release (never before; quality results never affect device work)",
            "command": f"python3 -I -B {ANALYZE} <evidence-dir>",
            "analyzer": ANALYZER_REL,
            "entry": f"{ANALYZE} binds analyze_filter.py's plan check to this namespace's plan.json",
            "note": ("Numerical gates and the original quality gates (N6 authored, N7, N8 stripes ROI) are all "
                     "recorded; the predicted N6 and N8-perspective failures are reported as failed quality, "
                     "never as a device-work failure. N6T is a diagnostic."),
        },
        "budgets": {
            "gpu_children_max": children,
            "children_planned": {"controls": len(CONTROLS), "numeric": len(numeric), "appearance": 1,
                                 "focused_retries_reserved": 0},
            "existing_frames_max": 0, "new_frames_max": NEW_FRAMES_MAX, "combined_frames_max": NEW_FRAMES_MAX,
            "existing_frames_planned": 0,
            "new_frames_planned": {"controls": controls_frames, "numeric": numeric_frames,
                                   "appearance": appearance["renders"], "total": planned,
                                   "unallocated": NEW_FRAMES_MAX - planned},
            "pngs_max": 32, "pairs_max": 16, "max_extent": [960, 540],
            "execution_seconds_max": 2700, "build_jobs_max": 2,
        },
        "timeouts_s": {"control": {"filter_device": 60, "ao_device": 120, "ao_device_cooked": 60},
                       "numeric": 150, "appearance": 300},
        "timeouts_basis": "Watchdogs, not measurements; their sum (1290 s) stays under the 2700 s ceiling.",
        "environment": BUILD004_PLAN["environment"],
        "shared_gpu_lock": BUILD004_PLAN["shared_gpu_lock"],
        "reviewed_repo_cargo_config_sha256": hashlib.sha256(repo_config.read_bytes()).hexdigest(),
        "grants": {
            "build": {"format": f"incant-{fc.NAMESPACE}-build-grant-v1", "required": [
                "packet", "namespace", "kind=build", "source_commit", "engine_commit", "plan_sha256",
                "helper_digest", f"evidence_dir (an existing <EVIDENCE_ROOT>/{fc.NAMESPACE})", "cargo_target_dir",
                "max_jobs<=2", "approved_cargo_configs {absolute path: sha256}"]},
            "gpu": {"format": f"incant-{fc.NAMESPACE}-gpu-grant-v1", "required": [
                "packet", "namespace", "kind=gpu", "source_commit", "engine_commit", "plan_sha256", "helper_digest",
                f"evidence_dir (the same {fc.NAMESPACE} directory)", "build_record_sha256",
                "expected_backend (Metal|Vulkan|Dx12)"]},
        },
        "sequence": [
            f"{RUNNER} build --grant <evidence>/build-grant.json   (includes the CPU guard regression)",
            f"{RUNNER} release-build --grant <evidence>/build-grant.json",
            *[f"{RUNNER} control --grant <evidence>/gpu-grant.json {c['name']}" for c in CONTROLS],
            *[f"{RUNNER} case --grant <evidence>/gpu-grant.json {c['name']}" for c in numeric],
            f"{RUNNER} appearance --grant <evidence>/gpu-grant.json",
            f"{RUNNER} release --grant <evidence>/gpu-grant.json   (always, also after any failure)",
            f"python3 -I -B {ANALYZE} <evidence>",
        ],
    }


def check_consistency(plan):
    problems = []
    if [(c["name"], c["frames"], c["stage_frames"]) for c in plan["numeric_cases"]] != \
            [(c["name"], c["frames"], c["stage_frames"]) for c in CASES["numeric_cases"]]:
        problems.append("numeric cases differ from ssao-repair/filter/cases.json")
    if plan["appearance"] != BUILD004_PLAN["appearance"]:
        problems.append("appearance differs from build004's")
    if plan["budgets"]["new_frames_planned"]["total"] != 100 or plan["budgets"]["new_frames_planned"]["unallocated"] < 0:
        problems.append(f"new submissions {plan['budgets']['new_frames_planned']} are not the approved 100 within 128")
    if plan["appearance"]["pngs"] != 16 or plan["appearance"]["pairs"] != 11:
        problems.append("appearance is not 16 PNGs / 11 pairs")
    if problems:
        raise SystemExit("; ".join(problems))


def main():
    argv = sys.argv[1:]
    current = json.loads(fc.PLAN.read_text()) if fc.PLAN.exists() else {}
    engine = current.get("engine_commit")
    if "--engine-commit" in argv:
        engine = argv[argv.index("--engine-commit") + 1]
        if len(engine) != 40 or any(ch not in "0123456789abcdef" for ch in engine):
            raise SystemExit("--engine-commit needs a full 40-hex commit")
    if engine is not None:
        verify_engine(engine)
    plan = build(engine)
    check_consistency(plan)
    text = json.dumps(plan, indent=1) + "\n"
    if "--check" in argv:
        if fc.PLAN.read_text() != text:
            raise SystemExit("plan.json differs from the source-derived plan")
        print("plan.json matches the source-derived plan")
        return
    fc.PLAN.write_text(text)
    print(json.dumps({"engine_commit": engine, "new_frames_planned": plan["budgets"]["new_frames_planned"],
                      "children": plan["budgets"]["gpu_children_max"]}, indent=1))


if __name__ == "__main__":
    main()
