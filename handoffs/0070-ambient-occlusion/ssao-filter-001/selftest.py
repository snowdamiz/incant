#!/usr/bin/env python3
"""Focused CPU guards for the 0070-ssao-filter-001 binding (no build, no GPU).

Covers only what this namespace adds to the reused build004 orchestration:
plan reproducibility and budgets, the unfrozen-engine and grant refusals,
stage-plane index validation, the feature binding, controls-before-cases
ordering, latching of a failed control by the reused ledger, the exact scope
of the helper copy's changes, and the digest coverage. Scratch stays inside
this namespace and is removed; the shared GPU lock is never touched.

  python3 -I -B selftest.py
"""

import difflib
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "tools")]
import filter_common as fc  # noqa: E402
import filter_runner as fr  # noqa: E402
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
    assert plan["engine_commit"] is None
    return plan


def freeze_and_grant_controls(plan, scratch):
    out = [refuses(fc.source_freeze, "0" * 40, contains="no frozen engine_commit")]
    evidence_root = scratch / "evidence"
    evidence = evidence_root / fc.NAMESPACE
    evidence.mkdir(parents=True)
    saved = (fc.base.EVIDENCE_ROOT, fc.base.REQUIRE_EXTERNAL_EVIDENCE)
    fc.base.EVIDENCE_ROOT, fc.base.REQUIRE_EXTERNAL_EVIDENCE = evidence_root, False
    try:
        good = {"format": f"incant-{fc.NAMESPACE}-gpu-grant-v1", "packet": fc.base.PACKET, "namespace": fc.NAMESPACE,
                "kind": "gpu", "source_commit": "0" * 40, "engine_commit": None, "evidence_dir": str(evidence),
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


def helper_scope():
    """The copy differs from build004's tools/effects only where documented."""
    old, new = fc.BASE_TOOLS / "effects", fc.HELPER
    report = {}
    for name in sorted({p.relative_to(old).as_posix() for p in (old / "src").glob("*.rs")} | {"Cargo.toml"}):
        a, b = (old / name).read_text().splitlines(), (new / name).read_text().splitlines()
        diff = [line for line in difflib.unified_diff(a, b, lineterm="", n=0) if line[:1] in "+-"
                and not line.startswith(("+++", "---"))]
        report[name] = {"added": sum(1 for x in diff if x.startswith("+")),
                        "removed": sum(1 for x in diff if x.startswith("-"))}
    unchanged = {"src/appearance.rs", "src/composed.rs", "src/gltf.rs"}
    assert all(report[n] == {"added": 0, "removed": 0} for n in unchanged), report
    numeric_old = (old / "src" / "numeric.rs").read_text()
    numeric_new = (new / "src" / "numeric.rs").read_text()
    # Undo the documented edits: the capture renames and the one changed message.
    renamed = numeric_new.replace("run.draw_with_stages(", "run.draw(").replace("run.submit_with_stages(", "run.submit(")
    renamed = renamed.replace("the filter plan names five", "the frozen plan names five")
    remaining = iter(renamed.splitlines())
    assert all(line in remaining for line in numeric_old.splitlines()), \
        "build004 numeric.rs is not an in-order subsequence of the copy: something besides insertions changed"
    assert numeric_new.count("run.draw_with_stages(") + numeric_new.count("run.submit_with_stages(") == 13, \
        "12 build004 call sites + 1 ssao_quality call site"
    return report


def digest_controls():
    digest, files = fc.helper_digest()
    reused = {f"{fc.base.TOOLS_REL}/{n}" for n in fc.REUSED}
    analyzer = {f"{fc.REPAIR_REL}/{n}" for n in fc.ANALYZER}
    own = {f"{fc.NAMESPACE_REL}/{n}" for n in ("plan.json", "filter_common.py", "filter_runner.py", "make_plan.py",
                                               "selftest.py", "effects/Cargo.toml", "effects/src/numeric.rs")}
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
                          "16 PNGs, 11 pairs; engine_commit unfrozen",
                  "freeze_and_grants": freeze_and_grant_controls(plan, scratch),
                  "stage_index": index_controls(scratch),
                  "feature_binding": feature_controls(plan),
                  "order_and_latch": order_and_latch_controls(plan, scratch),
                  "helper_scope": helper_scope(),
                  "digest": digest_controls(),
                  "cli": cli_controls()}
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    print(json.dumps(result, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
