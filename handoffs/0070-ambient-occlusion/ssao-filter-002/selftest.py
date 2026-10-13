#!/usr/bin/env python3
"""Focused CPU guards for the 0070-ssao-filter-002 binding (no build, no GPU).

The 0070-ssao-filter-001 controls, rebound to this namespace: plan
reproducibility and budgets, the unfrozen-engine and grant refusals (a 001
grant included), stage-plane index validation, the feature binding,
controls-before-cases ordering, latching of a failed control by the reused
ledger and the digest coverage. Added for 002: the helper copy differs from
001's only by the corrected sphere-coverage predicate, its unchanged 10%
tolerance and the synthetic regression; a Python twin of that regression's
fixture shows it rejects 001's predicate and accepts the corrected one; the
build's guard step refuses any other test set. Added for C2: the plan binds
root's frozen C2 source and the C2 phase; --engine-commit refuses the R1/R3
shader; reused analyzer bytes are pinned and a tampered copy is refused; the
C2 oracle with the R1/R3 phase reproduces the shared oracle on a sample. The
full C2 analyzer controls run separately (analyze.py --selftest). Scratch stays
inside this namespace and is removed; the shared GPU lock is never touched.

  python3 -I -B selftest.py
"""

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
sys.path[:0] = [str(HERE), str(HERE.parent / "tools")]
import filter_common as fc  # noqa: E402
import filter_runner as fr  # noqa: E402
import c2_analysis as c2  # noqa: E402
import c2_controls  # noqa: E402
import importlib.util  # noqa: E402

# By path: the shared analyzer puts tools/ (build004's own make_plan.py) ahead on sys.path.
_spec = importlib.util.spec_from_file_location("filter_002_make_plan", HERE / "make_plan.py")
make_plan = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(make_plan)
from ledger import Ledger, Refused  # noqa: E402


def refuses(function, *args, contains=""):
    try:
        function(*args)
    except Refused as refusal:
        assert contains in str(refusal), (contains, str(refusal))
        return str(refusal)
    raise AssertionError(f"{function.__name__} did not refuse")


def plan_controls():
    check = subprocess.run([sys.executable, "-I", "-B", str(HERE / "make_plan.py"), "--check"],
                           capture_output=True, text=True, timeout=120)
    assert check.returncode == 0, check.stderr[-400:]
    plan = fc.load_plan()
    b = plan["budgets"]
    assert b["new_frames_planned"] == {"controls": 9, "numeric": 75, "appearance": 16, "total": 100,
                                       "unallocated": 28}, b["new_frames_planned"]
    assert b["gpu_children_max"] == 9 and b["existing_frames_max"] == 0 and b["combined_frames_max"] == 128
    assert plan["appearance"]["pngs"] == 16 and plan["appearance"]["pairs"] == 11
    assert [c["name"] for c in plan["numeric_cases"]] == ["ssao_core", "ssao_materials", "ssao_geometry",
                                                          "lifecycle_drs", "ssao_quality"]
    assert sum(len(c["stage_frames"]) for c in plan["numeric_cases"]) == 15  # 13 build004 frames + 2 stripes
    engine = plan["engine_commit"]
    assert engine is None or (len(engine) == 40 and all(c in "0123456789abcdef" for c in engine))
    assert plan["namespace"] == "0070-ssao-filter-002" and plan["guard_regression"]["tests"] == list(fr.GUARD_TESTS)
    assert plan["analysis"]["command"].startswith(f"python3 -I -B {fc.NAMESPACE_REL}/analyze.py")
    assert plan["source_binding"]["engine_source_frozen"] == c2.ENGINE_SOURCE
    assert plan["analysis"]["sampling"]["sobol"] == list(c2.SOBOL)
    assert plan["analysis"]["sampling"]["reused_sources"] == c2.REUSED
    assert plan["analysis"]["quality"]["gates"] == sorted(c2.REVISED_GATES)
    assert "quality_n6_authored" not in plan["analysis"]["quality"]["gates"]
    assert plan["analysis"]["quality"]["retained_001"]["quality_n6_authored"]["pass"] is False
    return plan


def freeze_and_grant_controls(plan, scratch):
    saved_load = fc.load_plan
    try:
        fc.load_plan = lambda: {**plan, "engine_commit": None}
        out = [refuses(fc.source_freeze, "0" * 40, contains="no frozen engine_commit")]
        if plan["engine_commit"] is not None:
            fc.load_plan = lambda: plan
            assert fc.engine_commit() == plan["engine_commit"]
    finally:
        fc.load_plan = saved_load
    evidence_root = scratch / "evidence"
    evidence = evidence_root / fc.NAMESPACE
    evidence.mkdir(parents=True)
    saved = (fc.base.EVIDENCE_ROOT, fc.base.REQUIRE_EXTERNAL_EVIDENCE)
    fc.base.EVIDENCE_ROOT, fc.base.REQUIRE_EXTERNAL_EVIDENCE = evidence_root, False
    try:
        good = {"format": f"incant-{fc.NAMESPACE}-gpu-grant-v1", "packet": fc.base.PACKET, "namespace": fc.NAMESPACE,
                "kind": "gpu", "source_commit": "0" * 40, "engine_commit": plan["engine_commit"], "evidence_dir": str(evidence),
                "plan_sha256": fc.sha256_file(fc.PLAN), "helper_digest": fc.helper_digest()[0],
                "build_record_sha256": "0" * 64, "expected_backend": "Metal"}
        path = evidence / "gpu-grant.json"
        path.write_text(json.dumps(good))
        grant, _ = fc.load_grant(path, "gpu", plan)
        assert grant["namespace"] == fc.NAMESPACE
        for change, needle in (({"format": "incant-0070-gpu-grant-v1"}, "format"),
                               ({"plan_sha256": "0" * 64}, "plan_sha256"),
                               ({"engine_commit": "1" * 40}, "engine_commit"),
                               ({"helper_digest": "0" * 64}, "helper_digest")):
            path.write_text(json.dumps({**good, **change}))
            out.append(refuses(fc.load_grant, path, "gpu", plan, contains=needle))
        other = evidence_root / "0070-effects-build-004"
        other.mkdir()
        (other / "gpu-grant.json").write_text(json.dumps({**good, "evidence_dir": str(other)}))
        out.append(refuses(fc.load_grant, other / "gpu-grant.json", "gpu", plan, contains="namespace directory"))
        old = evidence_root / "0070-ssao-filter-001"
        old.mkdir()
        (old / "gpu-grant.json").write_text(json.dumps({**good, "format": "incant-0070-ssao-filter-001-gpu-grant-v1",
                                                         "namespace": "0070-ssao-filter-001", "evidence_dir": str(old)}))
        out.append(refuses(fc.load_grant, old / "gpu-grant.json", "gpu", plan, contains="0070-ssao-filter-001"))
    finally:
        fc.base.EVIDENCE_ROOT, fc.base.REQUIRE_EXTERNAL_EVIDENCE = saved
    return out


def write_output(directory, frames, planes, extra=None):
    directory.mkdir(parents=True)
    entries = []
    for label in frames:
        data = bytes(4 * 3 * 2)
        (directory / f"{label}.rgba").write_bytes(data)
        entries.append({"label": label, "file": f"{label}.rgba", "size": [3, 2],
                        "rgba_sha256": hashlib.sha256(data).hexdigest(), "meta": {}})
    for frame, stages in planes.items():
        for stage, data in stages.items():
            (directory / f"{frame}.{stage}.r8").write_bytes(data)
            entries.append({"label": f"{frame}.{stage}", "file": f"{frame}.{stage}.r8", "size": [3, 2],
                            "r8_sha256": hashlib.sha256(data).hexdigest(), "meta": {"frame": frame, "stage": stage}})
    for entry in extra or []:
        entries.append(entry)
    (directory / "index.json").write_text(json.dumps({"frames": entries, "summary": {}}))


def index_controls(scratch):
    plane = bytes(range(6))
    full = {"p_ao_x": {"raw": plane, "h": plane, "v": plane}}
    ok_dir = scratch / "index-ok"
    write_output(ok_dir, ["p_off_x", "p_ao_x"], full)
    index, problems = fr.validate_index(ok_dir, 0, [960, 540], ("p_ao_x",))
    assert not problems and index == {"frames": 2, "pngs": 0, "stage_frames": 1}, (index, problems)
    cases = {
        "missing plane": ({"p_ao_x": {"raw": plane, "h": plane}}, ("p_ao_x",), "planes ['h', 'raw']"),
        "undeclared frame": (full, (), "undeclared frame"),
        "short plane": ({"p_ao_x": {"raw": plane[:5], "h": plane, "v": plane}}, ("p_ao_x",), "5 bytes"),
        "declared frame without planes": ({}, ("p_ao_x",), "planes []"),
    }
    out = []
    for name, (planes, declared, needle) in cases.items():
        directory = scratch / f"index-{name.replace(' ', '-')}"
        write_output(directory, ["p_off_x", "p_ao_x"], planes)
        _, problems = fr.validate_index(directory, 0, [960, 540], declared)
        assert any(needle in p for p in problems), (name, problems)
        out.append(name)
    tampered = scratch / "index-tampered"
    write_output(tampered, ["p_ao_x"], full)
    (tampered / "p_ao_x.v.r8").write_bytes(bytes(6))
    _, problems = fr.validate_index(tampered, 0, [960, 540], ("p_ao_x",))
    assert any("r8 hash mismatch" in p for p in problems), problems
    out.append("tampered plane")
    return out


def feature_controls(plan):
    lib = {"features": ["default", "physics-deterministic", "ssao-stage-readback"]}
    cooked = {"features": ["cooked-geometry", "cooked-materials", "cooked-output", "default",
                           "physics-deterministic", "ssao-stage-readback"]}
    unit = "path+file:///x/crates/incant_render#0.0.1"
    helper = {unit: ["default,physics-deterministic,ssao-stage-readback"]}
    binding = fr.feature_binding(plan, {"lib": lib, "lib-cooked": cooked}, helper)
    assert not binding["problems"]
    out = []
    for name, frozen, units, needle in (
            ("readback feature absent", {"lib": {"features": ["default", "physics-deterministic"]}},
             {unit: ["default,physics-deterministic"]}, "lib features"),
            ("helper differs", {"lib": lib}, {unit: ["default,physics-deterministic"]}, "differ"),
            ("cooked in the default build", {"lib": {"features": lib["features"] + ["cooked-output"]}},
             {unit: ["cooked-output,default,physics-deterministic,ssao-stage-readback"]}, "cooked-* family"),
            ("cooked control without cooked-materials",
             {"lib": lib, "lib-cooked": {"features": ["default", "physics-deterministic", "ssao-stage-readback"]}},
             helper, "cooked control binary")):
        out.append(refuses(fr.feature_binding, plan, frozen, units, contains=needle))
        assert name
    return out


def order_and_latch_controls(plan, scratch):
    ledger = Ledger(scratch / "ledger")
    ledger.evidence.mkdir()
    with ledger.locked():
        refuses(fr.require_order, ledger, plan, "case", contains="control-filter_device")
        for name in ("filter_device", "ao_device"):
            ledger.append({"type": "reserve", "label": f"control-{name}", "kind": "suite"})
            ledger.append({"type": "complete", "label": f"control-{name}", "success": True,
                           "outcome": {"group_absent": True, "exit": 0}})
        fr.require_order(ledger, plan, "case")
        refuses(fr.require_order, ledger, plan, "appearance", contains="case-ssao_core")
        assert not ledger.failures()
        ledger.append({"type": "reserve", "label": "control-ao_device_cooked", "kind": "suite"})
        ledger.append({"type": "complete", "label": "control-ao_device_cooked", "success": False,
                       "failure_reasons": ["exit status 101"], "outcome": {"group_absent": True, "exit": 101}})
        assert list(ledger.failures()) == ["control-ao_device_cooked"], ledger.failures()
    return ["cases refused before both required controls", "appearance refused before every case",
            "a failed control latches in the reused ledger"]


PREDICATE_001 = """            let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            cx >= x0 && cy >= y0 && cx < x0 + w && cy < y0 + h && frame.px(x, y) != background
"""
PREDICATE_002 = """            let (fx, fy) = (f64::from(x), f64::from(y));
            fx >= x0
                && fy >= y0
                && fx + 1. <= x0 + w
                && fy + 1. <= y0 + h
                && frame.px(x, y) != background
"""
DOC_001 = "/// Viewport texels whose colour differs from the background texel (16, 20).\n"
DOC_002 = """/// Texels wholly inside the viewport whose colour differs from the background
/// texel (16, 20). The fractional viewport's partly covered rim (columns 13
/// and 293, row 17) blends with the canvas and is never sphere coverage.
"""
COVERS = """/// The isolated sphere's coverage precondition, unchanged: within 10% of its
/// analytic silhouette area.
const SPHERE_AREA_TOLERANCE: f64 = 0.1;
fn sphere_covers(covered: usize, expected: f64) -> bool {
    (covered as f64 - expected).abs() <= SPHERE_AREA_TOLERANCE * expected
}
"""
CHECK_001 = "            (covered as f64 - expected).abs() <= 0.1 * expected,\n"
CHECK_002 = "            sphere_covers(covered, expected),\n"
TESTS_MARK = "\n/// Synthetic regression for the sphere-coverage precondition"


def helper_scope():
    """The copy differs from 0070-ssao-filter-001's helper only where documented:
    numeric.rs reverts to 001's byte for byte once the predicate, the named
    tolerance and the test module are undone; elsewhere only comments differ."""
    old, new = PACKET / "ssao-filter-001" / "effects", fc.HELPER
    report = {}
    for name in ("src/appearance.rs", "src/common.rs", "src/composed.rs", "src/gltf.rs"):
        assert (old / name).read_bytes() == (new / name).read_bytes(), name
        report[name] = "identical"
    for name, marker in (("src/lib.rs", "//"), ("src/main.rs", "//"), ("Cargo.toml", "#")):
        a, b = (old / name).read_text().splitlines(), (new / name).read_text().splitlines()
        code = lambda lines: [x for x in lines if not x.lstrip().startswith(marker)]  # noqa: E731
        assert code(a) == code(b), f"{name}: a non-comment line changed"
        report[name] = "comments only"
    numeric = (new / "src" / "numeric.rs").read_text()
    tests = numeric[numeric.index(TESTS_MARK):]
    assert tests.count("#[test]") == 3 and all(n.rsplit("::", 1)[1] in tests for n in fr.GUARD_TESTS)
    reverted = numeric[:numeric.index(TESTS_MARK)]
    for done, undone in ((PREDICATE_002, PREDICATE_001), (DOC_002, DOC_001), (COVERS, ""), (CHECK_002, CHECK_001)):
        assert reverted.count(done) == 1, done[:60]
        reverted = reverted.replace(done, undone)
    assert reverted == (old / "src" / "numeric.rs").read_text(), "numeric.rs differs from 001 beyond the documented edits"
    report["src/numeric.rs"] = "001 + corrected predicate + named 10% tolerance + 3 synthetic tests"
    return report


def sphere_area(orthographic):
    """numeric.rs sphere_area, transcribed."""
    radius, half_height = 0.5 * 1.2, 160.5 / 2
    pixels = radius * half_height / 4 if orthographic else math.tan(math.asin(radius / 3.0)) * half_height / math.tan(0.5)
    return math.pi * pixels * pixels


def guard_controls():
    """Python twin of the Rust sphere_guard_tests fixture (the Rust tests
    themselves run at the 002 build): the fixture rejects 001's centre test and
    accepts the corrected wholly-inside test; and the build's guard step refuses
    any libtest output other than exactly the three named tests passing."""
    x0, y0, w, h = 13.25, 17.5, 280.5, 160.5
    width, height = 321, 193
    canvas, backdrop = (31, 63, 95, 255), (20, 21, 25, 255)
    column_rim, row_rim, disc_px = (22, 32, 45, 255), (26, 47, 70, 255), (128, 128, 128, 255)

    def frame(radius):
        cx, cy = x0 + w / 2, y0 + h / 2
        out = {}
        for y in range(height):
            for x in range(width):
                overlaps = x + 1 > x0 and y + 1 > y0 and x < x0 + w and y < y0 + h
                whole = x >= x0 and y >= y0 and x + 1 <= x0 + w and y + 1 <= y0 + h
                disc = radius is not None and (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 < radius * radius
                out[(x, y)] = (canvas if not overlaps else (row_rim if y < y0 else column_rim) if not whole
                               else disc_px if disc else backdrop)
        return out

    def centre_001(f):
        bg = f[(16, 20)]
        return sum(1 for (x, y), p in f.items()
                   if x + 0.5 >= x0 and y + 0.5 >= y0 and x + 0.5 < x0 + w and y + 0.5 < y0 + h and p != bg)

    def whole_002(f):
        bg = f[(16, 20)]
        return sum(1 for (x, y), p in f.items()
                   if x >= x0 and y >= y0 and x + 1 <= x0 + w and y + 1 <= y0 + h and p != bg)

    covers = lambda covered, expected: abs(covered - expected) <= 0.1 * expected  # noqa: E731
    rim = frame(None)
    assert sum(1 for p in rim.values() if p in (column_rim, row_rim)) == 601
    assert centre_001(rim) == 601 and whole_002(rim) == 0
    out = {"rim_only": {"001_predicate": 601, "002_predicate": 0}}
    for orthographic in (False, True):
        expected = sphere_area(orthographic)
        disc = frame(math.sqrt(expected / math.pi))
        painted = sum(1 for p in disc.values() if p == disc_px)
        assert whole_002(disc) == painted and centre_001(disc) == painted + 601 and covers(painted, expected)
        assert not covers(centre_001(disc), expected), "001's predicate must fail the disc fixture"
        out["ortho" if orthographic else "persp"] = {"expected": round(expected, 3), "painted": painted,
                                                    "001_predicate": painted + 601, "002_predicate": painted}
    expected = sphere_area(False)
    assert covers(2543, expected) and covers(3107, expected) and not covers(2541, expected) \
        and not covers(3109, expected) and not covers(3419, expected) and covers(2818, expected)
    names = list(fr.GUARD_TESTS)
    ok = "".join(f"test {n} ... ok\n" for n in names) + "\ntest result: ok. 3 passed; 0 failed; 0 ignored;\n"
    assert fr.guard_verdict(ok)["passed"] == sorted(names)
    refusals = []
    for label, log in (("no test ran (filter matched nothing)", "\ntest result: ok. 0 passed; 0 failed;\n"),
                       ("a test missing", "".join(f"test {n} ... ok\n" for n in names[:2])
                        + "test result: ok. 2 passed; 0 failed;\n"),
                       ("an extra test", ok.replace("test result: ok. 3", "test numeric::x ... ok\ntest result: ok. 4")),
                       ("a failed test", ok.replace("test result: ok. 3 passed; 0 failed",
                                                    "test result: FAILED. 2 passed; 1 failed"))):
        refuses(fr.guard_verdict, log, contains="guard regression")
        refusals.append(label)
    out["guard_step_refuses"] = refusals
    return out


def c2_controls_fast(scratch):
    """C2 binding checks that need no oracle sweep (the full set: analyze.py --selftest)."""
    out = {"phase": c2_controls.phase_controls(), "binding": c2_controls.binding_controls(scratch)}
    try:
        make_plan.verify_engine(c2.ENGINE_SOURCE["base"])
        raise AssertionError("verify_engine accepted the R1/R3 engine")
    except SystemExit as refusal:
        assert "is not the C2 shader" in str(refusal), str(refusal)
        out["engine_pin_refuses_r1r3"] = str(refusal)[:160]

    class Through(c2.C2FilterOracle):
        @staticmethod
        def phase(x, y, ray):
            return c2.orc.jitter(x, y)

    scene = c2.orc.scene("stripes", False)
    shared, through = c2.orc.FilterOracle(scene), Through(scene)
    sample = c2.af.INTERIOR[::211]
    for x, y in sample:
        for surface in scene.candidates((x, y)):
            if surface is not None:
                assert through.k_interval(x, y, surface) == shared.k_interval(x, y, surface), (x, y)
    out["fidelity_sample_texels"] = len(sample)
    return out


def digest_controls():
    digest, files = fc.helper_digest()
    reused = {f"{fc.base.TOOLS_REL}/{n}" for n in fc.REUSED}
    analyzer = {f"{fc.REPAIR_REL}/{n}" for n in fc.ANALYZER}
    own = {f"{fc.NAMESPACE_REL}/{n}" for n in ("plan.json", "filter_common.py", "filter_runner.py", "make_plan.py",
                                               "selftest.py", "analyze.py", "c2_analysis.py",
                                               "c2_controls.py", "effects/Cargo.toml", "effects/src/numeric.rs")}
    assert reused | analyzer | own <= set(files), sorted((reused | analyzer | own) - set(files))
    assert fc.HELPER_LOCK_REL not in files
    return {"digest": digest, "files": len(files), "reused_tools": len(reused), "analyzer": len(analyzer)}


def cli_controls():
    absent = subprocess.run([sys.executable, "-I", "-B", str(HERE / "filter_runner.py"), "build",
                             "--grant", str(HERE / ".no-such-grant.json")], capture_output=True, text=True,
                            timeout=120)
    assert absent.returncode == 2 and "absent" in absent.stderr, (absent.returncode, absent.stderr[-300:])
    return ["build without a grant refuses (exit 2) before any process or lock"]


def main():
    scratch = HERE / f".selftest-{os.getpid()}"
    scratch.mkdir()
    try:
        plan = plan_controls()
        result = {"plan": "make_plan --check reproduces plan.json; 100 of 128 new submissions; 9 children; "
                          "16 PNGs, 11 pairs; frozen and unfrozen engine guards retained; guard tests bound",
                  "freeze_and_grants": freeze_and_grant_controls(plan, scratch),
                  "stage_index": index_controls(scratch),
                  "feature_binding": feature_controls(plan),
                  "order_and_latch": order_and_latch_controls(plan, scratch),
                  "helper_scope": helper_scope(),
                  "sphere_guard": guard_controls(),
                  "c2": c2_controls_fast(scratch),
                  "digest": digest_controls(),
                  "cli": cli_controls()}
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    print(json.dumps(result, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
