#!/usr/bin/env python3
"""Guarded serial runner for the filter execution namespace 0070-ssao-filter-002.

Standard library only. It reuses the released build004 supervised orchestration
unchanged by import (tools/runner.py, lifecycle.py, ledger.py, common.py):
shared GPU lock -> grant preflight -> packet lock -> fsynced reservation ->
spawn behind the identity gate -> watchdog -> kill, verify and reap -> completion
record -> latch on any failure. Only the namespace bindings differ: its plan,
helper crate, frozen engine commit, grant formats and digest
(filter_common.py), the build's binaries and feature binding, two exact
ignored controls instead of a suite, stage-plane validation of the helper
outputs, and an ordering guard (controls, then cases, then appearance).
The build also runs the helper's synthetic sphere-coverage regression (CPU
only) and records nothing unless exactly its three tests pass.

A failed child latches: every later device command refuses. There is no retry
command in this namespace. The offline analysis (quality included) runs after
release and never affects device work.

  python3 -I -B filter_runner.py build         --grant <evidence>/build-grant.json
  python3 -I -B filter_runner.py release-build --grant <evidence>/build-grant.json
  python3 -I -B filter_runner.py control       --grant <evidence>/gpu-grant.json <control>
  python3 -I -B filter_runner.py case          --grant <evidence>/gpu-grant.json <numeric-case>
  python3 -I -B filter_runner.py appearance    --grant <evidence>/gpu-grant.json
  python3 -I -B filter_runner.py release       --grant <evidence>/gpu-grant.json
  python3 -I -B filter_runner.py reconcile     --grant <evidence>/<gpu-or-build-grant>.json
  python3 -I -B filter_runner.py status        --grant <evidence>/<grant>.json
"""

import argparse
import json
import re
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "tools")]
import filter_common as fc  # noqa: E402
import lifecycle  # noqa: E402
import runner as base  # noqa: E402  (tools/runner.py, reused unchanged)
from ledger import Ledger, Refused  # noqa: E402

# Bind the reused orchestration to this namespace.
base.c = fc
base.HELPER_PACKAGE = fc.HELPER_PACKAGE
STAGES = ("raw", "h", "v")
STAGE_LABEL = re.compile(r"(?P<frame>[A-Za-z0-9_@-]+)\.(?P<stage>raw|h|v)")
GUARD_TESTS = tuple(f"numeric::sphere_guard_tests::{name}" for name in (
    "sphere_guard_fractional_rim_alone_is_not_sphere",
    "sphere_guard_interior_disc_counts_within_tolerance",
    "sphere_guard_tolerance_stays_ten_percent"))


# ------------------------------------------------------------------ build

def feature_binding(plan, frozen, helper_units):
    """The lib test binary and the helper's incant_render share one exact
    feature set, which includes ssao-stage-readback and no cooked-* family;
    the cooked control binary carries cooked-materials. Recorded, then enforced."""
    expected = plan["features"]
    helper_ids = [k for k in helper_units if re.search(r"/crates/incant_render#", k)]
    helper = None
    if len(helper_ids) == 1 and len(helper_units[helper_ids[0]]) == 1:
        helper = sorted(f for f in helper_units[helper_ids[0]][0].split(",") if f)
    binding = {"lib": sorted(frozen["lib"]["features"]), "helper_incant_render": helper,
               "lib_cooked": sorted(frozen["lib-cooked"]["features"]) if "lib-cooked" in frozen else None,
               "expected": expected}
    problems = []
    if helper is None:
        problems.append(f"expected one incant_render unit in the helper build, found {helper_ids}")
    elif binding["lib"] != helper:
        problems.append("lib test and helper incant_render features differ")
    if binding["lib"] != expected["lib_expected"]:
        problems.append(f"lib features {binding['lib']} != plan {expected['lib_expected']}")
    if any(f.startswith("cooked") for f in binding["lib"] + (helper or [])):
        problems.append("a cooked-* family is enabled in the default lib or helper build")
    if "lib-cooked" in frozen and not set(expected["lib_cooked_required"]) <= set(binding["lib_cooked"]):
        problems.append(f"cooked control binary lacks {expected['lib_cooked_required']}")
    binding["problems"] = problems
    if problems:
        raise Refused(f"feature binding refused: {problems}")
    return binding


def guard_verdict(log):
    """Exactly GUARD_TESTS ran and passed in this libtest output, or refuse."""
    passed = sorted(re.findall(r"^test (\S+) \.\.\. ok$", log, re.M))
    summary = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed", log, re.M)
    if passed != sorted(GUARD_TESTS) or summary != [("ok", str(len(GUARD_TESTS)), "0")]:
        raise Refused(f"guard regression did not pass exactly {list(GUARD_TESTS)}: {passed} {summary}")
    return {"passed": passed, "summary": list(summary[0])}


def guard_regression(ledger, plan, env, jobs, manifest):
    """The helper's synthetic sphere-coverage regression, CPU only: exactly
    GUARD_TESTS must run and pass before a build record exists."""
    base.cargo(ledger, plan, env, jobs, "helper-guard-tests", ["test", *manifest, "--lib", "--", "sphere_guard"])
    return guard_verdict((ledger.evidence / "logs" / "build-helper-guard-tests.stdout").read_text(errors="replace"))


def build(args, plan):
    """Build lease only: the packet lock, no shared GPU lock, no device work."""
    grant, grant_sha = fc.load_grant(args.grant, "build", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    with ledger.locked():
        if ledger.blocked():
            raise Refused(f"process groups never verified absent: {sorted(ledger.blocked())}; run reconcile")
        if (evidence / "build" / "build.json").exists():
            raise Refused("build.json exists; frozen builds are never replaced")
        freeze = fc.source_freeze(grant["source_commit"])
        digest, helper_files = fc.helper_digest()
        tmp = evidence / "tmp" / "build"
        tmp.mkdir(parents=True, exist_ok=True)
        env = fc.build_env(grant["cargo_target_dir"], tmp)
        audit = fc.cargo_config_audit(env, plan, grant.get("approved_cargo_configs", {}))
        compilers = fc.compiler_inputs(env)
        jobs = grant["max_jobs"]
        if not (fc.HELPER / "Cargo.lock").exists():
            shutil.copyfile(fc.ROOT / "Cargo.lock", fc.HELPER / "Cargo.lock")
            argv = [str(fc.ROOT / "tools/cargo"), "metadata", "--offline", "--format-version", "1",
                    "--manifest-path", str(fc.HELPER / "Cargo.toml")]
            lock_env = {**env, "TMPDIR": str(base.private_dir(ledger, "tmp", "build-helper-lock"))}
            done = base.execute(ledger, plan, {"label": "build-helper-lock", "kind": "build", "counts_as_child": False},
                                argv, lock_env, fc.ROOT, base.BUILD_TIMEOUT_S)
            if done["success"] is not True:
                raise Refused("cargo metadata could not prune the helper lock offline")
        lock = base.verify_helper_lock()
        features = plan["features"]
        lib = base.cargo(ledger, plan, env, jobs, "lib-tests",
                         ["test", "--no-run", "-p", "incant_render", "--lib",
                          "--features", ",".join(features["lib_requested"])])
        manifest = ["--manifest-path", str(fc.HELPER / "Cargo.toml")]
        helper = base.cargo(ledger, plan, env, jobs, "helper", ["build", *manifest, "--bin", "effects_gpu"])
        clippy = base.cargo(ledger, plan, env, jobs, "helper-clippy",
                            ["clippy", *manifest, "--all-targets", "--", "-D", "warnings"])
        guard = guard_regression(ledger, plan, env, jobs, manifest)
        lib_test = lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render"  # noqa: E731
        frozen = {"lib": base.freeze_binary(evidence, lib, "lib", lib_test),
                  "effects_gpu": base.freeze_binary(evidence, helper, "effects_gpu",
                                                    lambda a: a["target"] == "effects_gpu")}
        if any(ctl["binary"] == "lib-cooked" for ctl in plan["controls"]):
            cooked = base.cargo(ledger, plan, env, jobs, "lib-cooked-tests",
                                ["test", "--no-run", "-p", "incant_render", "--lib",
                                 "--features", ",".join(features["lib_cooked_requested"])])
            frozen["lib-cooked"] = base.freeze_binary(evidence, cooked, "lib-cooked", lib_test)
        helper_units = base.unit_features(helper)
        binding = feature_binding(plan, frozen, helper_units)
        after = fc.source_freeze(grant["source_commit"])
        before_paths = {d["path"] for d in freeze["dirty_or_untracked"]} | {fc.HELPER_LOCK_REL}
        changed = [d["path"] for d in after["dirty_or_untracked"] if d["path"] not in before_paths]
        unchanged = {d["path"]: d["sha256"] for d in freeze["dirty_or_untracked"]}
        drifted = [d["path"] for d in after["dirty_or_untracked"]
                   if d["path"] in unchanged and d["sha256"] != unchanged[d["path"]]]
        if fc.helper_digest()[0] != digest or changed or drifted:
            raise Refused(f"helper bytes or working tree changed during the build: {changed + drifted}")
        if fc.compiler_inputs(env) != compilers \
                or fc.cargo_config_audit(env, plan, grant.get("approved_cargo_configs", {})) != audit:
            raise Refused("compiler binaries or Cargo configuration changed during the build")
        record = {"format": f"incant-{fc.NAMESPACE}-build-v1", "namespace": fc.NAMESPACE, "grant_sha256": grant_sha,
                  "grant": grant, "source_before": freeze, "source_after": after, "helper_digest": digest,
                  "helper_files": helper_files, "tool_revision": fc.tool_revision(), "helper_lock": lock,
                  "cargo_config_audit": audit, "compiler_inputs": compilers, "host": fc.host_facts(),
                  "frozen": frozen, "feature_binding": binding,
                  "unit_features": {"helper": helper_units, "lib_tests": base.unit_features(lib)},
                  "clippy_artifacts": len(clippy), "guard_regression": guard,
                  "plan_sha256": fc.sha256_file(fc.PLAN),
                  "recorded": base.now_iso()}
        base.write_new_json(evidence / "build" / "build.json", record)
        print(json.dumps({"build_record_sha256": fc.sha256_file(evidence / "build" / "build.json"),
                          "frozen": {k: v["sha256"] for k, v in frozen.items()},
                          "features": {k: binding[k] for k in ("lib", "helper_incant_render", "lib_cooked")}},
                         indent=1))


# ------------------------------------------------------------------ verdicts

def validate_index(out_dir, pngs_expected, max_extent, stage_frames):
    """tools/runner.validate_index plus R8 stage planes: every declared stage
    frame has exactly its raw, h and v planes, hash- and size-exact; no plane
    belongs to an undeclared frame; every file is listed."""
    index_path = out_dir / "index.json"
    if not index_path.is_file():
        return {"frames": 0, "pngs": 0, "stage_frames": 0}, ["index.json absent"]
    try:
        entries = json.loads(index_path.read_text())["frames"]
    except (ValueError, KeyError, TypeError):
        return {"frames": 0, "pngs": 0, "stage_frames": 0}, ["index.json is not a frame index"]
    if not isinstance(entries, list) or not entries:
        return {"frames": 0, "pngs": 0, "stage_frames": 0}, ["index lists no frames"]
    problems, listed, labels, pngs, rgba, planes = [], {"index.json"}, set(), 0, set(), {}
    for entry in entries:
        label = entry.get("label") if isinstance(entry, dict) else None
        if not isinstance(label, str) or not base.SAFE_LABEL.fullmatch(label) or label in labels:
            problems.append(f"missing, unsafe or duplicated label {label!r}")
            continue
        labels.add(label)
        name, size = entry.get("file"), entry.get("size")
        stage = STAGE_LABEL.fullmatch(label)
        allowed = (f"{label}.r8",) if stage else (f"{label}.rgba", f"{label}.png")
        if name not in allowed or not (out_dir / name).is_file():
            problems.append(f"{label}: file {name!r} absent or misnamed")
            continue
        listed.add(name)
        if not (isinstance(size, list) and len(size) == 2 and all(isinstance(v, int) and v > 0 for v in size)):
            problems.append(f"{label}: invalid size {size!r}")
            continue
        digest, length = fc.sha256_file(out_dir / name), (out_dir / name).stat().st_size
        if stage:
            meta = entry.get("meta") or {}
            if stage["frame"] not in stage_frames:
                problems.append(f"{label}: stage plane of an undeclared frame")
            if meta.get("frame") != stage["frame"] or meta.get("stage") != stage["stage"]:
                problems.append(f"{label}: stage metadata {meta!r} does not match its label")
            if length != size[0] * size[1]:
                problems.append(f"{label}: {length} bytes, expected {size[0] * size[1]}")
            if digest != entry.get("r8_sha256"):
                problems.append(f"{label}: r8 hash mismatch")
            planes.setdefault(stage["frame"], set()).add(stage["stage"])
        elif name.endswith(".rgba"):
            rgba.add(label)
            if length != size[0] * size[1] * 4:
                problems.append(f"{label}: byte length does not match {size}")
            if digest != entry.get("rgba_sha256"):
                problems.append(f"{label}: rgba hash mismatch")
        else:
            pngs += 1
            if digest != entry.get("png_sha256"):
                problems.append(f"{label}: png hash mismatch")
            if size[0] > max_extent[0] or size[1] > max_extent[1]:
                problems.append(f"{label}: {size} exceeds {max_extent}")
    for frame in stage_frames:
        if frame not in rgba:
            problems.append(f"stage frame {frame}: RGBA frame absent")
        if planes.get(frame, set()) != set(STAGES):
            problems.append(f"stage frame {frame}: planes {sorted(planes.get(frame, set()))}, expected {list(STAGES)}")
    present = {p.relative_to(out_dir).as_posix() for p in out_dir.rglob("*") if p.is_file()}
    if present - listed:
        problems.append(f"files not in the index: {sorted(present - listed)[:10]}")
    if pngs != pngs_expected:
        problems.append(f"{pngs} PNGs; expected {pngs_expected}")
    return {"frames": len(rgba) + pngs, "pngs": pngs, "stage_frames": len(planes)}, problems


def receipt_problems(stdout, case, declared):
    """tools/runner.helper_postprocess's receipt rules, unchanged: exactly one
    budget receipt within the declaration and one verdict with every check ok."""
    receipts, malformed = [], 0
    for line in stdout.read_text(errors="replace").splitlines():
        if line.startswith(base.RECEIPT):
            try:
                receipt = json.loads(line[len(base.RECEIPT):])
            except ValueError:
                malformed += 1
                continue
            receipts.append(receipt if isinstance(receipt, dict) else {})
        elif base.RECEIPT.strip() in line:
            malformed += 1
    problems = [f"{malformed} malformed receipt line(s)"] if malformed else []
    foreign = sorted({str(r.get("case")) for r in receipts} - {case})
    if foreign:
        problems.append(f"receipts for another case {foreign}")
    budgets = [r.get("body") for r in receipts if r.get("kind") == "budget"]
    verdicts = [r.get("body") for r in receipts if r.get("kind") == "verdict"]
    frames = None
    if len(budgets) != 1 or not isinstance(budgets[0], dict):
        problems.append(f"{len(budgets)} budget receipt(s); exactly one is required")
    else:
        frames, labels = budgets[0].get("frames"), budgets[0].get("labels")
        if budgets[0].get("declared") != declared:
            problems.append(f"declared {budgets[0].get('declared')} != planned {declared}")
        if not isinstance(frames, int) or not 0 < frames <= declared:
            problems.append(f"frames {frames!r} outside 1..{declared}")
        if not isinstance(labels, list) or len(labels) != frames or len(set(map(str, labels))) != len(labels):
            problems.append("frame labels missing, duplicated or not matching the frame count")
    if len(verdicts) != 1 or not isinstance(verdicts[0], dict):
        problems.append(f"{len(verdicts)} verdict receipt(s); exactly one is required")
    else:
        checks = verdicts[0].get("checks")
        if verdicts[0].get("failures") != []:
            problems.append(f"verdict failures {verdicts[0].get('failures')}")
        if not isinstance(checks, list) or not checks or \
                not all(isinstance(x, dict) and x.get("ok") is True for x in checks):
            problems.append("verdict has no checks or a check that is not ok")
    failures = [v.get("failures") for v in verdicts if isinstance(v, dict)]
    return receipts, frames, failures, problems


def helper_postprocess(evidence, case, declared, pngs, out_dir, max_extent, stage_frames):
    """Receipts as in tools/runner.helper_postprocess; the index with stage planes."""
    def postprocess(_outcome, stdout, _stderr):
        receipts, frames, failures, problems = receipt_problems(stdout, case, declared)
        index, index_problems = validate_index(out_dir, pngs, max_extent, stage_frames) if out_dir.exists() \
            else ({"frames": 0, "pngs": 0, "stage_frames": 0}, ["output directory absent"])
        problems = problems + index_problems
        files = {}
        if out_dir.exists():
            for path in sorted(p for p in out_dir.rglob("*") if p.is_file()):
                files[str(path.relative_to(evidence))] = fc.sha256_file(path)
        return {"receipts": len(receipts), "frames": frames, "declared": declared, "index": index,
                "stage_frames": list(stage_frames), "verdict_failures": failures, "files": files,
                "valid": not problems, "problems": problems}
    return postprocess


# ------------------------------------------------------------------ device commands

def control_spec(plan, name):
    controls = {ctl["name"]: ctl for ctl in plan["controls"]}
    if name not in controls:
        raise Refused(f"unknown control {name}")
    return controls[name]


def succeeded(ledger, label):
    done = ledger.children().get(label, {}).get("complete")
    return bool(done) and done.get("success") is True


def require_order(ledger, plan, kind):
    """Controls before cases; every case before appearance."""
    missing = [f"control-{c['name']}" for c in plan["controls"]
               if c["required_before_cases"] and not succeeded(ledger, f"control-{c['name']}")]
    if kind == "appearance":
        missing += [f"case-{c['name']}" for c in plan["numeric_cases"] if not succeeded(ledger, f"case-{c['name']}")]
    if missing:
        raise Refused(f"{kind} must follow successful {missing}")


def run_control(session, plan, record, source, ctl):
    if ctl["binary"] not in record["frozen"]:
        raise Refused(f"control {ctl['name']}: binary {ctl['binary']} was not frozen by the build")
    binary, label = record["frozen"][ctl["binary"]], f"control-{ctl['name']}"
    # Kind "suite" (an exact ignored-test run) so the reused ledger latches a failure.
    return base.execute(session.ledger, plan, {"label": label, "kind": "suite", "control": ctl["name"],
                                               "target": ctl["binary"], "tests": [ctl["test"]],
                                               "existing_frames": 0, "new_frames": ctl["frames"], "source": source},
                        [binary["path"], "--ignored", "--exact", "--test-threads=1", ctl["test"]],
                        base.child_env(session.ledger, session.grant, label),
                        base.private_dir(session.ledger, "cwd", label), plan["timeouts_s"]["control"][ctl["name"]],
                        binary={"path": binary["path"], "sha256": binary["sha256"]},
                        postprocess=base.suite_postprocess([ctl["test"]]), inherit_fds=(session.shared_fd,))


def run_helper(session, plan, record, source, target, label, kind, timeout):
    if target == "appearance":
        declared, pngs, stage_frames, tail = plan["appearance"]["renders"], plan["appearance"]["pngs"], (), \
            ["appearance"]
    else:
        case = base.numeric_case(plan, target)
        declared, pngs, stage_frames, tail = case["frames"], 0, tuple(case["stage_frames"]), ["case", target]
    binary = record["frozen"]["effects_gpu"]
    out_dir = session.evidence / "outputs" / label
    if out_dir.exists():
        raise Refused(f"{out_dir} exists; outputs are never replaced")
    out_dir.parent.mkdir(parents=True, exist_ok=True)
    return base.execute(session.ledger, plan, {"label": label, "kind": kind, "target": target,
                                               "new_frames": declared, "pngs": pngs, "source": source},
                        [binary["path"], *tail, str(out_dir)], base.child_env(session.ledger, session.grant, label),
                        base.private_dir(session.ledger, "cwd", label), timeout,
                        binary={"path": binary["path"], "sha256": binary["sha256"]},
                        postprocess=helper_postprocess(session.evidence, target, declared, pngs, out_dir,
                                                       plan["budgets"]["max_extent"], stage_frames),
                        inherit_fds=(session.shared_fd,))


def command_control(args, plan):
    ctl = control_spec(plan, args.target)
    with base.device_session(args, plan) as session:
        record, source = base.gpu_checks(session)
        return run_control(session, plan, record, source, ctl)


def command_case(args, plan):
    case = base.numeric_case(plan, args.target)
    with base.device_session(args, plan) as session:
        record, source = base.gpu_checks(session)
        require_order(session.ledger, plan, "case")
        return run_helper(session, plan, record, source, case["name"], f"case-{case['name']}", "numeric",
                          plan["timeouts_s"]["numeric"])


def command_appearance(args, plan):
    with base.device_session(args, plan) as session:
        record, source = base.gpu_checks(session)
        require_order(session.ledger, plan, "appearance")
        return run_helper(session, plan, record, source, "appearance", "appearance", "appearance",
                          plan["timeouts_s"]["appearance"])


COMMANDS = {"build": build, "release-build": base.command_release_build, "control": command_control,
            "case": command_case, "appearance": command_appearance, "release": base.command_release,
            "reconcile": base.command_reconcile, "status": base.command_status}


def parse(argv):
    parser = argparse.ArgumentParser(description=f"{fc.NAMESPACE} guarded runner", add_help=True)
    parser.add_argument("command", choices=sorted(COMMANDS))
    parser.add_argument("--grant", required=True)
    parser.add_argument("target", nargs="?")
    return parser.parse_args(argv)


def run_command(argv, plan):
    """Exit status: 0 success, 1 a child failed (latched), 2 refused."""
    args = parse(argv)
    try:
        completion = COMMANDS[args.command](args, plan)
    except Refused as refusal:
        base.record_refusal(args.grant, str(refusal), argv)
        print(f"refused: {refusal}", file=sys.stderr)
        return 2
    if isinstance(completion, dict) and completion.get("type") == "complete" and completion.get("success") is not True:
        print(f"child {completion['label']} failed {completion.get('failure_reasons')}; its evidence and "
              "reservation are retained and the namespace is latched", file=sys.stderr)
        return 1
    return 0


def main(argv=None):
    lifecycle.install_signal_handlers()
    return run_command(sys.argv[1:] if argv is None else argv, fc.load_plan())


if __name__ == "__main__":
    sys.exit(main())
