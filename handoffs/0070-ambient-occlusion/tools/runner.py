#!/usr/bin/env python3
"""Guarded serial runner for handoff 0070 (combined SSAO + bloom). Stdlib only.

Nothing runs without a root-written grant. Build and GPU grants are separate
files in the root-assigned evidence directory; either may exist alone.

Device commands (list, suite, case, appearance, retry, release) first take the
shared native GPU lock (EVIDENCE_ROOT/native-gpu.lock, flock, non-waiting),
then load the grant, then take this packet's lock, and hold both until the
command ends: preflight -> fsynced reservation -> spawn behind the identity
gate -> watchdog -> finally kill, verify and reap the group -> completion
record (even after postprocessing errors) -> unlock. Children inherit the
shared-lock descriptor, so no other packet can render while any owned process
lives. Refusals never start a process.

Every device completion is classified. Anything but an explicit success
(nonzero exit, signal, timeout, interruption, unverified group, changed binary,
missing or extra tests, a missing or failed verdict, an invalid frame receipt
or index) latches the packet: all further device work refuses except a
root-receipted precise retry of a latched child. `reconcile` signals a group
only while a live member matches an exact identity (pid + microsecond start
time) this packet recorded; otherwise it signals nothing, reports, and every
child stays refused.

  python3 -I -B runner.py build         --grant <evidence>/build-grant.json
  python3 -I -B runner.py release-build --grant <evidence>/build-grant.json
  python3 -I -B runner.py list          --grant <evidence>/gpu-grant.json <suite>
  python3 -I -B runner.py suite         --grant <evidence>/gpu-grant.json <suite>
  python3 -I -B runner.py case          --grant <evidence>/gpu-grant.json <numeric-case>
  python3 -I -B runner.py appearance    --grant <evidence>/gpu-grant.json
  python3 -I -B runner.py retry         --grant <evidence>/gpu-grant.json --receipt <evidence>/<file>.json
  python3 -I -B runner.py release       --grant <evidence>/gpu-grant.json
  python3 -I -B runner.py reconcile     --grant <evidence>/<gpu-or-build-grant>.json
  python3 -I -B runner.py status        --grant <evidence>/<grant>.json

Suites: lib, model_gpu, gpu_host-game, gpu_host-ui (listing must match first).
"""

import argparse
import json
import os
import re
import shutil
import signal
import sys
import time
from contextlib import contextmanager
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402
import lifecycle  # noqa: E402
from ledger import Ledger, Refused, exclusive, now_iso  # noqa: E402

BUILD_TIMEOUT_S = 3600
SUITE_BINARIES = ("lib", "model_gpu", "gpu_host-game", "gpu_host-ui")
HELPER_PACKAGE = "incant_0070_effects"
RECEIPT = "EFFECTS-RECEIPT "
RETRY_FORMAT = "incant-0070-retry-v2"
SAFE_LABEL = re.compile(r"[A-Za-z0-9_.@-]+")
RECONCILE_GRACE_S = 2.0
RECONCILE_ABSENT_WAIT_S = 10.0


def write_new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "x") as out:
        json.dump(value, out, indent=1, sort_keys=True)
        out.write("\n")


def record_refusal(grant_path, reason, argv):
    """Append a refusal only inside a plausible evidence directory: the grant
    file must sit directly in an existing directory under EVIDENCE_ROOT."""
    try:
        grant_path = Path(grant_path).resolve()
        evidence = Path(json.loads(grant_path.read_text()).get("evidence_dir", "")).resolve()
    except (OSError, ValueError, TypeError, AttributeError):
        return
    if not evidence.is_dir() or c.EVIDENCE_ROOT not in evidence.parents or grant_path.parent != evidence:
        return
    line = json.dumps({"at": now_iso(), "reason": reason, "argv": argv}, sort_keys=True) + "\n"
    with open(evidence / "refusals.jsonl", "a") as out:
        out.write(line)
        out.flush()


# ------------------------------------------------------------------ locks and preflight

class Session:
    """A device command's verified context; valid while both locks are held."""

    def __init__(self, grant, grant_sha, evidence, ledger, shared_fd):
        self.grant, self.grant_sha, self.evidence = grant, grant_sha, evidence
        self.ledger, self.shared_fd = ledger, shared_fd


@contextmanager
def device_session(args, plan):
    """Shared native GPU lock first, then the grant preflight, then this
    packet's lock. Both are non-waiting and held until the command returns."""
    with exclusive(c.shared_gpu_lock_path(), "the shared native GPU lock") as shared_fd:
        grant, grant_sha = c.load_grant(args.grant, "gpu", plan)
        evidence = Path(grant["evidence_dir"])
        ledger = Ledger(evidence)
        with ledger.locked():
            yield Session(grant, grant_sha, evidence, ledger, shared_fd)


def gpu_checks(session, retry_of=None):
    """Re-verified under both locks before every device child."""
    evidence, ledger, grant = session.evidence, session.ledger, session.grant
    if (evidence / "released.json").exists():
        raise Refused("the GPU lease was released by this packet; no further GPU work")
    blocked = ledger.blocked()
    if blocked:
        raise Refused(f"process groups never verified absent: {sorted(blocked)}; run reconcile "
                      "(it signals only groups proven by an exact recorded identity)")
    failures = ledger.failures()
    if failures and retry_of is None:
        raise Refused(f"latched failure(s) {sorted(failures)}: further device work is refused; only a "
                      "root-receipted precise retry of a latched child may run")
    if retry_of is not None and retry_of not in failures:
        raise Refused(f"{retry_of} is not a latched failure of this packet")
    build_record = evidence / "build" / "build.json"
    if not build_record.is_file() or c.sha256_file(build_record) != grant.get("build_record_sha256"):
        raise Refused("build record absent or not the one pinned by the GPU grant")
    record = json.loads(build_record.read_text())
    freeze = c.source_freeze(grant["source_commit"])
    digest = c.helper_digest()[0]
    if record["helper_digest"] != digest or record["source_before"]["head"] != freeze["head"]:
        raise Refused("source or helper bytes differ from the frozen build")
    if not (c.HELPER / "Cargo.lock").is_file() or \
            c.sha256_file(c.HELPER / "Cargo.lock") != record["helper_lock"]["helper_lock_sha256"]:
        raise Refused("helper Cargo.lock differs from the frozen build")
    return record, {"grant_sha256": session.grant_sha, "head": freeze["head"], "tree": freeze["tree"],
                    "dirty_or_untracked": freeze["dirty_or_untracked"], "helper_digest": digest}


# ------------------------------------------------------------------ execution

def assess(outcome, completion, needs_verdict):
    """Failure reasons for one child. Only an empty list is a success."""
    reasons = []
    if completion.get("error"):
        reasons.append(f"runner error: {completion['error']}")
    if outcome.get("interrupted"):
        reasons.append(f"interrupted: {outcome['interrupted']}")
    if outcome.get("gate_opened") is not True:
        reasons.append("the spawn gate never opened; the program did not run")
    if outcome.get("timed_out"):
        reasons.append(f"watchdog timeout after {outcome.get('timeout_s')}s")
    if outcome.get("signal"):
        reasons.append(f"terminated by {outcome['signal']}")
    elif outcome.get("exit") != 0:
        reasons.append(f"exit status {outcome.get('exit')}")
    if outcome.get("group_absent") is not True or outcome.get("survivors") or outcome.get("leader_unreaped"):
        reasons.append("process group not verified absent")
    if outcome.get("cleanup_error"):
        reasons.append(f"cleanup error: {outcome['cleanup_error']}")
    if outcome.get("observe_errors"):
        reasons.append("member identities could not all be recorded")
    if completion.get("binary_unchanged") is False:
        reasons.append("frozen binary changed during the run")
    if completion.get("postprocess_error"):
        reasons.append(f"postprocess error: {completion['postprocess_error']}")
    if needs_verdict:
        post = completion.get("postprocess")
        if not isinstance(post, dict) or post.get("valid") is not True:
            problems = post.get("problems") if isinstance(post, dict) else None
            reasons.append(f"no valid verdict: {problems or 'postprocess result absent'}")
    return reasons


def execute(ledger, plan, request, argv, env, cwd, timeout_s, binary=None, postprocess=None, inherit_fds=()):
    """Reserve, run and record one child. The caller holds the packet lock and,
    for device work, the shared GPU lock whose descriptor the child inherits.
    Returns the completion; re-raises only an interruption, after recording."""
    label = request["label"]
    try:
        lifecycle.identity(os.getpid())
    except OSError as error:
        raise Refused(f"process identity unsupported here: {error}") from error
    logs = ledger.evidence / "logs"
    logs.mkdir(exist_ok=True)
    stdout, stderr = logs / f"{label}.stdout", logs / f"{label}.stderr"
    if stdout.exists() or stderr.exists():
        raise Refused(f"logs for {label} already exist; outputs are never replaced")
    if binary is not None and c.sha256_file(binary["path"]) != binary["sha256"]:
        raise Refused(f"{binary['path']} does not match its frozen hash")
    private = [Path(p) for p in (cwd, env.get("TMPDIR")) if p and Path(p).is_relative_to(ledger.evidence)]
    if any(p.exists() for p in private):
        raise Refused(f"private directories for {label} already exist")
    ledger.check_and_reserve(plan["budgets"], {**request, "timeout_s": timeout_s,
                                               "argv": [str(a) for a in argv],
                                               "env": c.recorded_env(env), "cwd": str(cwd),
                                               "binary": binary, "inherited_fds": len(inherit_fds)})
    for path in private:
        path.mkdir(parents=True)
    outcome, error = {}, None
    completion = {"type": "complete", "label": label}
    try:
        lifecycle.run(argv, env=env, cwd=cwd, stdout_path=stdout, stderr_path=stderr,
                      timeout_s=timeout_s, outcome=outcome, inherit_fds=inherit_fds,
                      on_spawn=lambda pid, pgid, ident: ledger.append(
                          {"type": "spawned", "label": label, "pid": pid, "pgid": pgid, "identity": ident}),
                      on_member=lambda ident: ledger.append(
                          {"type": "member", "label": label, "identity": ident}))
    except BaseException as exc:  # spawn failure or interruption; the group is already handled
        error = exc
    finally:
        completion["outcome"] = outcome
        completion["error"] = None if error is None else f"{type(error).__name__}: {error}"
        for name, path in (("stdout", stdout), ("stderr", stderr)):
            completion[f"{name}_sha256"] = c.sha256_file(path) if path.exists() else None
            completion[f"{name}_bytes"] = path.stat().st_size if path.exists() else None
        if binary is not None:
            completion["binary_unchanged"] = c.sha256_file(binary["path"]) == binary["sha256"]
        if error is None and postprocess is not None:
            try:
                completion["postprocess"] = postprocess(outcome, stdout, stderr)
            except Exception as exc:
                completion["postprocess_error"] = f"{type(exc).__name__}: {exc}"
        reasons = assess(outcome, completion, needs_verdict=postprocess is not None)
        completion["success"] = not reasons
        completion["failure_reasons"] = reasons
        ledger.append(completion)
    if error is not None and not isinstance(error, Exception):
        raise error  # KeyboardInterrupt or Terminated, recorded above
    print(json.dumps({"label": label, "success": completion["success"], "pid": outcome.get("pid"),
                      "exit": outcome.get("exit"), "signal": outcome.get("signal"),
                      "timed_out": outcome.get("timed_out"), "group_absent": outcome.get("group_absent"),
                      "failure_reasons": reasons}))
    return completion


def private_dir(ledger, kind, label):
    """Path of a per-child private directory; execute() creates it after reserving."""
    return ledger.evidence / kind / label


# ------------------------------------------------------------------ build

def split_cargo_args(args):
    if "--" in args:
        index = args.index("--")
        return args[:index], args[index:]
    return args, []


def cargo(ledger, plan, env, jobs, label, args):
    head, tail = split_cargo_args(args)
    argv = [str(c.ROOT / "tools/cargo"), *head, "--locked", "--offline", "--profile", "ci", f"-j{jobs}",
            "--message-format=json-render-diagnostics", *tail]
    completion = execute(ledger, plan, {"label": f"build-{label}", "kind": "build", "counts_as_child": False},
                         argv, env, c.ROOT, BUILD_TIMEOUT_S)
    if completion["success"] is not True:
        raise Refused(f"cargo {label} failed {completion['failure_reasons']}; see logs/build-{label}.stderr")
    artifacts = []
    for line in (ledger.evidence / "logs" / f"build-{label}.stdout").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact":
            artifacts.append({"target": message["target"]["name"], "kind": message["target"]["kind"],
                              "test": message["profile"]["test"], "fresh": message["fresh"],
                              "executable": message.get("executable"), "features": message.get("features", []),
                              "package_id": message["package_id"], "opt_level": message["profile"]["opt_level"]})
    return artifacts


def lock_packages(text):
    packages = {}
    for block in re.split(r"\n(?=\[\[)", text):
        if not block.startswith("[[package]]"):
            continue
        fields = dict(re.findall(r'^(name|version|source|checksum) = "([^"]*)"', block, re.M))
        deps = re.search(r"^dependencies = \[\n(.*?)^\]", block, re.M | re.S)
        refs = re.findall(r'^ "([^"]*)",$', deps.group(1), re.M) if deps else []
        packages[(fields["name"], fields["version"], fields.get("source"))] = (fields.get("checksum"), refs)
    return packages


def verify_helper_lock():
    """The helper lock must be a pruned copy of the repository lock."""
    root = lock_packages((c.ROOT / "Cargo.lock").read_text())
    helper = lock_packages((c.HELPER / "Cargo.lock").read_text())
    own = [k for k in helper if k[0] == HELPER_PACKAGE]
    foreign = [f"{k[0]} {k[1]}" for k in helper if k[0] != HELPER_PACKAGE and k not in root]
    checksum = [f"{k[0]} {k[1]}" for k in helper if k in root and helper[k][0] != root[k][0]]
    added = {f"{k[0]} {k[1]}": sorted(set(helper[k][1]) - set(root[k][1]))
             for k in helper if k in root and set(helper[k][1]) - set(root[k][1])}
    record = {"repository_lock_sha256": c.sha256_file(c.ROOT / "Cargo.lock"),
              "helper_lock_sha256": c.sha256_file(c.HELPER / "Cargo.lock"),
              "repository_packages": len(root), "helper_packages": len(helper),
              "own_entries": [list(k) for k in own], "not_in_repository_lock": foreign,
              "checksum_mismatches": checksum, "dependency_edges_added": added}
    if len(own) != 1 or foreign or checksum or added:
        raise Refused(f"helper Cargo.lock is not a pruned repository lock: {json.dumps(record)[:800]}")
    return record


def unit_features(artifacts):
    out = {}
    for a in artifacts:
        if a["test"] or not set(a["kind"]) & {"lib", "rlib", "proc-macro"}:
            continue
        out.setdefault(a["package_id"], set()).add(",".join(sorted(a["features"])))
    return {k: sorted(v) for k, v in sorted(out.items())}


def package_id(features, crate):
    found = [k for k in features if re.search(rf"/crates/{crate}#", k)]
    if len(found) != 1:
        raise Refused(f"expected one {crate} package id, found {found}")
    return found[0]


def freeze_binary(evidence, artifacts, name, match):
    found = [a for a in artifacts if a["executable"] and match(a)]
    if len(found) != 1:
        raise Refused(f"{name}: expected exactly one executable artifact, found {len(found)}")
    source = Path(found[0]["executable"])
    target = evidence / "bin" / name
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        raise Refused(f"{target} exists; frozen binaries are never replaced")
    shutil.copyfile(source, target)
    target.chmod(0o555)
    digest = c.sha256_file(target)
    if digest != c.sha256_file(source):
        raise Refused(f"copy mismatch for {name}")
    return {"path": str(target), "sha256": digest, "bytes": target.stat().st_size, "built_from": str(source),
            "fresh": found[0]["fresh"], "features": found[0]["features"], "package_id": found[0]["package_id"]}


def build(args, plan):
    """Build lease only: the packet lock, no shared GPU lock, no device work."""
    grant, grant_sha = c.load_grant(args.grant, "build", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    with ledger.locked():
        if ledger.blocked():
            raise Refused(f"process groups never verified absent: {sorted(ledger.blocked())}; run reconcile")
        if (evidence / "build" / "build.json").exists():
            raise Refused("build.json exists; frozen builds are never replaced")
        freeze = c.source_freeze(grant["source_commit"])
        digest, helper_files = c.helper_digest()
        tmp = evidence / "tmp" / "build"
        tmp.mkdir(parents=True, exist_ok=True)
        env = c.build_env(grant["cargo_target_dir"], tmp)
        audit = c.cargo_config_audit(env, plan, grant.get("approved_cargo_configs", {}))
        compilers = c.compiler_inputs(env)
        jobs = grant["max_jobs"]
        if not (c.HELPER / "Cargo.lock").exists():
            shutil.copyfile(c.ROOT / "Cargo.lock", c.HELPER / "Cargo.lock")
            argv = [str(c.ROOT / "tools/cargo"), "metadata", "--offline", "--format-version", "1",
                    "--manifest-path", str(c.HELPER / "Cargo.toml")]
            done = execute(ledger, plan, {"label": "build-helper-lock", "kind": "build", "counts_as_child": False},
                           argv, env, c.ROOT, BUILD_TIMEOUT_S)
            if done["success"] is not True:
                raise Refused("cargo metadata could not prune the helper lock offline")
        lock = verify_helper_lock()
        tests = cargo(ledger, plan, env, jobs, "engine-tests",
                      ["test", "--no-run", "-p", "incant_render", "-p", "incant_render_gpu_host"])
        manifest = ["--manifest-path", str(c.HELPER / "Cargo.toml")]
        helper = cargo(ledger, plan, env, jobs, "helper", ["build", *manifest, "--bin", "effects_gpu"])
        clippy = cargo(ledger, plan, env, jobs, "helper-clippy",
                       ["clippy", *manifest, "--all-targets", "--", "-D", "warnings"])
        frozen = {
            "lib": freeze_binary(evidence, tests, "lib", lambda a: a["test"] and a["kind"] == ["lib"]
                                 and a["target"] == "incant_render"),
            "model_gpu": freeze_binary(evidence, tests, "model_gpu", lambda a: a["test"] and a["target"] == "model_gpu"),
            "gpu_host-game": freeze_binary(evidence, tests, "gpu_host-game",
                                           lambda a: a["test"] and a["target"] == "game"),
            "gpu_host-ui": freeze_binary(evidence, tests, "gpu_host-ui", lambda a: a["test"] and a["target"] == "ui"),
            "effects_gpu": freeze_binary(evidence, helper, "effects_gpu", lambda a: a["target"] == "effects_gpu"),
        }
        regression, helper_units = unit_features(tests), unit_features(helper)
        identity = {}
        for crate in ("incant_render", "incant_render_gpu_host"):
            reg = regression[package_id(regression, crate)]
            hel = helper_units.get(package_id(regression, crate))
            identity[crate] = {"regression": reg, "helper": hel, "equal": reg == hel}
            if reg != hel or any("cooked" in f for f in ",".join(reg).split(",")):
                raise Refused(f"{crate} feature set differs or enables a cooked family: {identity[crate]}")
        shared = sorted(set(regression) & set(helper_units))
        after = c.source_freeze(grant["source_commit"])
        before_paths = {d["path"] for d in freeze["dirty_or_untracked"]} | {c.HELPER_LOCK_REL}
        changed = [d["path"] for d in after["dirty_or_untracked"] if d["path"] not in before_paths]
        unchanged = {d["path"]: d["sha256"] for d in freeze["dirty_or_untracked"]}
        drifted = [d["path"] for d in after["dirty_or_untracked"]
                   if d["path"] in unchanged and d["sha256"] != unchanged[d["path"]]]
        if c.helper_digest()[0] != digest or changed or drifted:
            raise Refused(f"helper bytes or working tree changed during the build: {changed + drifted}")
        if c.compiler_inputs(env) != compilers \
                or c.cargo_config_audit(env, plan, grant.get("approved_cargo_configs", {})) != audit:
            raise Refused("compiler binaries or Cargo configuration changed during the build")
        record = {"format": "incant-0070-build-v1", "grant_sha256": grant_sha, "grant": grant,
                  "source_before": freeze, "source_after": after, "helper_digest": digest,
                  "helper_files": helper_files, "helper_lock": lock, "cargo_config_audit": audit,
                  "compiler_inputs": compilers, "host": c.host_facts(), "frozen": frozen,
                  "unit_features": {"regression": regression, "helper": helper_units},
                  "feature_identity": identity,
                  "feature_differences": {p: {"regression": regression[p], "helper": helper_units[p]}
                                          for p in shared if regression[p] != helper_units[p]},
                  "clippy_artifacts": len(clippy), "plan_sha256": c.sha256_file(c.PLAN),
                  "recorded": now_iso()}
        write_new_json(evidence / "build" / "build.json", record)
        print(json.dumps({"build_record_sha256": c.sha256_file(evidence / "build" / "build.json"),
                          "frozen": {k: v["sha256"] for k, v in frozen.items()},
                          "feature_identity": {k: v["equal"] for k, v in identity.items()}}, indent=1))


# ------------------------------------------------------------------ verdicts

def listing_postprocess(evidence, suite, expected, label):
    def postprocess(_outcome, stdout, _stderr):
        text = stdout.read_text(errors="replace")
        names = re.findall(r"^(\S+): test$", text, re.M)
        counts = re.findall(r"^(\d+) tests?, (\d+) benchmarks?$", text, re.M)
        missing, unexpected = sorted(set(expected) - set(names)), sorted(set(names) - set(expected))
        duplicates = sorted({n for n in names if names.count(n) > 1})
        problems = []
        if missing:
            problems.append(f"missing {missing}")
        if unexpected:
            problems.append(f"unexpected {unexpected}")
        if duplicates:
            problems.append(f"duplicated {duplicates}")
        if counts != [(str(len(expected)), "0")]:
            problems.append(f"libtest count line {counts}; expected {len(expected)} tests, 0 benchmarks")
        result = {"listed": len(names), "expected": len(expected), "missing": missing, "unexpected": unexpected,
                  "duplicates": duplicates, "match": not problems, "valid": not problems, "problems": problems}
        write_new_json(evidence / "listings" / f"{label}.json", {"suite": suite, "names": names, **result})
        return result
    return postprocess


def suite_postprocess(expected):
    def postprocess(_outcome, stdout, _stderr):
        text = stdout.read_text(errors="replace")
        lines = re.findall(r"^test (\S+) \.\.\. (\S+)$", text, re.M)
        results, duplicates = {}, []
        for name, status in lines:
            if name in results:
                duplicates.append(name)
            results[name] = status
        running = [int(n) for n in re.findall(r"^running (\d+) tests?$", text, re.M)]
        summary = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                             r"(\d+) measured; (\d+) filtered out", text, re.M)
        missing, unexpected = sorted(set(expected) - set(results)), sorted(set(results) - set(expected))
        failed = sorted(n for n, s in results.items() if n in expected and s != "ok")
        problems = []
        if missing:
            problems.append(f"missing {missing}")
        if unexpected:
            problems.append(f"unexpected {unexpected}")
        if failed:
            problems.append(f"not ok {failed}")
        if duplicates:
            problems.append(f"reported twice {duplicates}")
        if running != [len(expected)]:
            problems.append(f"libtest announced {running}; expected [{len(expected)}]")
        if len(summary) != 1 or summary[0][:4] != ("ok", str(len(expected)), "0", "0"):
            problems.append(f"summary {summary}; expected exactly one 'ok. {len(expected)} passed; 0 failed; 0 ignored'")
        return {"expected": len(expected), "reported": len(results),
                "passed": sum(1 for s in results.values() if s == "ok"), "failed": failed, "missing": missing,
                "unexpected": unexpected, "duplicates": duplicates, "running": running, "summary": summary,
                "valid": not problems, "problems": problems}
    return postprocess


def validate_index(out_dir, pngs_expected, max_extent):
    """The helper's index must list exactly the kept files, hash-for-hash."""
    index_path = out_dir / "index.json"
    if not index_path.is_file():
        return {"frames": 0, "pngs": 0}, ["index.json absent"]
    try:
        frames = json.loads(index_path.read_text())["frames"]
    except (ValueError, KeyError, TypeError):
        return {"frames": 0, "pngs": 0}, ["index.json is not a frame index"]
    if not isinstance(frames, list) or not frames:
        return {"frames": 0, "pngs": 0}, ["index lists no frames"]
    problems, listed, labels, pngs = [], {"index.json"}, set(), 0
    for entry in frames:
        label = entry.get("label") if isinstance(entry, dict) else None
        if not isinstance(label, str) or not SAFE_LABEL.fullmatch(label) or label in labels:
            problems.append(f"missing, unsafe or duplicated label {label!r}")
            continue
        labels.add(label)
        name, size = entry.get("file"), entry.get("size")
        if name not in (f"{label}.rgba", f"{label}.png") or not (out_dir / name).is_file():
            problems.append(f"{label}: file {name!r} absent or misnamed")
            continue
        listed.add(name)
        if not (isinstance(size, list) and len(size) == 2 and all(isinstance(v, int) and v > 0 for v in size)):
            problems.append(f"{label}: invalid size {size!r}")
            continue
        digest = c.sha256_file(out_dir / name)
        if name.endswith(".rgba"):
            if (out_dir / name).stat().st_size != size[0] * size[1] * 4:
                problems.append(f"{label}: byte length does not match {size}")
            if digest != entry.get("rgba_sha256"):
                problems.append(f"{label}: rgba hash mismatch")
        else:
            pngs += 1
            if digest != entry.get("png_sha256"):
                problems.append(f"{label}: png hash mismatch")
            if size[0] > max_extent[0] or size[1] > max_extent[1]:
                problems.append(f"{label}: {size} exceeds {max_extent}")
    present = {p.relative_to(out_dir).as_posix() for p in out_dir.rglob("*") if p.is_file()}
    if present - listed:
        problems.append(f"files not in the index: {sorted(present - listed)[:10]}")
    if pngs != pngs_expected:
        problems.append(f"{pngs} PNGs; expected {pngs_expected}")
    return {"frames": len(frames), "pngs": pngs}, problems


def helper_postprocess(evidence, case, declared, pngs, out_dir, max_extent):
    def postprocess(_outcome, stdout, _stderr):
        receipts, malformed = [], 0
        for line in stdout.read_text(errors="replace").splitlines():
            if line.startswith(RECEIPT):
                try:
                    receipt = json.loads(line[len(RECEIPT):])
                except ValueError:
                    malformed += 1
                    continue
                receipts.append(receipt if isinstance(receipt, dict) else {})
            elif RECEIPT.strip() in line:
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
            budget = budgets[0]
            frames, labels = budget.get("frames"), budget.get("labels")
            if budget.get("declared") != declared:
                problems.append(f"declared {budget.get('declared')} != planned {declared}")
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
        index, index_problems = validate_index(out_dir, pngs, max_extent) if out_dir.exists() \
            else ({"frames": 0, "pngs": 0}, ["output directory absent"])
        problems.extend(index_problems)
        files = {}
        if out_dir.exists():
            for path in sorted(p for p in out_dir.rglob("*") if p.is_file()):
                files[str(path.relative_to(evidence))] = c.sha256_file(path)
        return {"receipts": len(receipts), "frames": frames, "declared": declared, "index": index,
                "verdict_failures": [v.get("failures") for v in verdicts if isinstance(v, dict)],
                "files": files, "valid": not problems, "problems": problems}
    return postprocess


# ------------------------------------------------------------------ device commands

def child_env(ledger, grant, label):
    return c.run_env(private_dir(ledger, "tmp", label), grant["expected_backend"])


def listing_matched(ledger, suite):
    done = ledger.children().get(f"list-{suite}", {}).get("complete")
    return bool(done) and done.get("success") is True and (done.get("postprocess") or {}).get("match") is True


def run_listing(session, plan, record, source, suite, label, kind, timeout, extra=None):
    binary = record["frozen"][suite]
    expected = plan["suites"][suite]["ignored"]
    return execute(session.ledger, plan, {"label": label, "kind": kind, "target": suite, "source": source,
                                          **(extra or {})},
                   [binary["path"], "--list", "--ignored"], child_env(session.ledger, session.grant, label),
                   private_dir(session.ledger, "cwd", label), timeout,
                   binary={"path": binary["path"], "sha256": binary["sha256"]},
                   postprocess=listing_postprocess(session.evidence, suite, expected, label),
                   inherit_fds=(session.shared_fd,))


def suite_frames(plan, suite, tests):
    entry = plan["suites"][suite]
    existing = sum(entry["existing"][n]["frames"] for n in tests if n in entry["existing"])
    new = sum(entry["new_in_suite"][n]["frames"] for n in tests if n in entry["new_in_suite"])
    return existing, new


def run_suite(session, plan, record, source, suite, tests, label, kind, timeout, extra=None):
    binary = record["frozen"][suite]
    existing, new = suite_frames(plan, suite, tests)
    return execute(session.ledger, plan, {"label": label, "kind": kind, "target": suite, "tests": tests,
                                          "existing_frames": existing, "new_frames": new, "source": source,
                                          **(extra or {})},
                   [binary["path"], "--ignored", "--exact", "--test-threads=1", *tests],
                   child_env(session.ledger, session.grant, label), private_dir(session.ledger, "cwd", label),
                   timeout, binary={"path": binary["path"], "sha256": binary["sha256"]},
                   postprocess=suite_postprocess(tests), inherit_fds=(session.shared_fd,))


def run_helper(session, plan, record, source, target, label, kind, timeout, extra=None):
    if target == "appearance":
        declared, pngs, tail = plan["appearance"]["renders"], plan["appearance"]["pngs"], ["appearance"]
    else:
        declared, pngs, tail = numeric_case(plan, target)["frames"], 0, ["case", target]
    binary = record["frozen"]["effects_gpu"]
    out_dir = session.evidence / "outputs" / label
    if out_dir.exists():
        raise Refused(f"{out_dir} exists; outputs are never replaced")
    out_dir.parent.mkdir(parents=True, exist_ok=True)
    return execute(session.ledger, plan, {"label": label, "kind": kind, "target": target, "new_frames": declared,
                                          "pngs": pngs, "source": source, **(extra or {})},
                   [binary["path"], *tail, str(out_dir)], child_env(session.ledger, session.grant, label),
                   private_dir(session.ledger, "cwd", label), timeout,
                   binary={"path": binary["path"], "sha256": binary["sha256"]},
                   postprocess=helper_postprocess(session.evidence, target, declared, pngs, out_dir,
                                                  plan["budgets"]["max_extent"]),
                   inherit_fds=(session.shared_fd,))


def numeric_case(plan, name):
    cases = {case["name"]: case for case in plan["numeric_cases"]}
    if name not in cases:
        raise Refused(f"unknown numeric case {name}")
    return cases[name]


def require_suite(suite):
    if suite not in SUITE_BINARIES:
        raise Refused(f"unknown suite {suite}")
    return suite


def command_list(args, plan):
    suite = require_suite(args.target)
    with device_session(args, plan) as session:
        record, source = gpu_checks(session)
        return run_listing(session, plan, record, source, suite, f"list-{suite}", "listing",
                           plan["timeouts_s"]["listing"])


def command_suite(args, plan):
    suite = require_suite(args.target)
    with device_session(args, plan) as session:
        record, source = gpu_checks(session)
        if not listing_matched(session.ledger, suite):
            raise Refused(f"{suite}: no successful matching listing; run `list {suite}` first")
        return run_suite(session, plan, record, source, suite, plan["suites"][suite]["run"], f"suite-{suite}",
                         "suite", plan["timeouts_s"]["suite"][suite])


def command_case(args, plan):
    case = numeric_case(plan, args.target)
    with device_session(args, plan) as session:
        record, source = gpu_checks(session)
        return run_helper(session, plan, record, source, case["name"], f"case-{case['name']}", "numeric",
                          plan["timeouts_s"]["numeric"])


def command_appearance(args, plan):
    with device_session(args, plan) as session:
        record, source = gpu_checks(session)
        return run_helper(session, plan, record, source, "appearance", "appearance", "appearance",
                          plan["timeouts_s"]["appearance"])


def load_retry_receipt(path, session):
    path = Path(path or "")
    if not path.is_absolute() or not path.is_file() or path.resolve().parent != session.evidence.resolve():
        raise Refused("a retry receipt must be a root-written file directly in the evidence directory")
    raw = path.read_bytes()
    try:
        receipt = json.loads(raw)
    except ValueError as error:
        raise Refused("retry receipt is not JSON") from error
    if not isinstance(receipt, dict):
        raise Refused("retry receipt is not a JSON object")
    expected = {"format": RETRY_FORMAT, "packet": c.PACKET, "approved_by": "root",
                "source_commit": session.grant["source_commit"]}
    problems = [f"{k}={receipt.get(k)!r}" for k, v in expected.items() if receipt.get(k) != v]
    for key in ("original_label", "kind", "target", "reason"):
        if not isinstance(receipt.get(key), str) or not receipt[key].strip():
            problems.append(f"{key} missing")
    if problems:
        raise Refused(f"retry receipt refused: {problems}")
    return receipt, c.sha256_bytes(raw), path


def command_retry(args, plan):
    """One precise retry of one latched child, named by a root receipt:
    same kind and target, and for suites only tests that did not pass."""
    with device_session(args, plan) as session:
        receipt, receipt_sha, path = load_retry_receipt(args.receipt, session)
        original = receipt["original_label"]
        children = session.ledger.children()
        if original not in children:
            raise Refused(f"retry receipt names unknown child {original}")
        record, source = gpu_checks(session, retry_of=original)
        base = children[original]["reserve"]
        kind, target = base.get("base_kind", base["kind"]), base.get("target")
        reserves = [e["reserve"] for e in children.values()]
        if any(r.get("retry_of") == original for r in reserves):
            raise Refused(f"{original} was already retried once")
        if any(r.get("receipt_sha256") == receipt_sha for r in reserves):
            raise Refused("this retry receipt was already used")
        if receipt["kind"] != kind or receipt["target"] != target:
            raise Refused(f"imprecise retry: the receipt names {receipt['kind']}/{receipt['target']}, "
                          f"the latched child is {kind}/{target}")
        label = f"retry-{original}"
        extra = {"base_kind": kind, "retry_of": original, "receipt_sha256": receipt_sha, "receipt": str(path),
                 "reason": receipt["reason"]}
        timeout = plan["timeouts_s"]["retry"]
        if kind == "listing":
            return run_listing(session, plan, record, source, target, label, "retry", timeout, extra)
        if kind == "suite":
            tests = receipt.get("tests")
            post = (children[original]["complete"] or {}).get("postprocess") or {}
            allowed = set(post.get("failed", [])) | set(post.get("missing", [])) if post else set(base["tests"])
            if not isinstance(tests, list) or not tests or len(set(tests)) != len(tests) \
                    or not set(tests) <= allowed:
                raise Refused(f"imprecise suite retry: tests must be a non-empty subset of {sorted(allowed)}")
            return run_suite(session, plan, record, source, target, sorted(tests), label, "retry", timeout, extra)
        if kind in ("numeric", "appearance"):
            return run_helper(session, plan, record, source, target, label, "retry", timeout, extra)
        raise Refused(f"no retry route for kind {kind}")


# ------------------------------------------------------------------ reconcile and release

def shared_lock_state():
    try:
        with exclusive(c.shared_gpu_lock_path(), "the shared native GPU lock"):
            return "free"
    except Refused:
        return "held"


def frozen_processes(evidence):
    """Live processes whose executable is a frozen binary of this packet."""
    prefix = str((evidence / "bin").resolve()) + "/"
    found = []
    for pid in lifecycle.all_pids():
        path = lifecycle.pid_path(pid)
        if path and path.startswith(prefix):
            found.append({"pid": pid, "path": path, "identity": lifecycle.identity(pid)})
    return found


def reconcile_child(entry, retained):
    """Stop a group only while it still holds a process this packet recorded.

    A live member whose pid, microsecond start time and uid match a retained
    identity proves the group was never recycled; its other members then
    belong to the same session lineage and are adopted. Start-time ordering
    alone proves nothing, so a group without such a member is never signalled.
    """
    if not entry["spawned"]:
        return {"resolved": True, "group_absent": None, "signals": [], "unproven": [], "survivors": [],
                "retained": [], "note": "no spawned record: the spawn gate never opened, so nothing executed"}
    pgid = entry["spawned"][-1]["pgid"]
    retained, adopted, signals = list(retained), [], []

    def snapshot():
        return [(pid, lifecycle.identity(pid)) for pid in lifecycle.group_pids(pgid)]

    def proven(members):
        return [i for _, i in members if i and i["pgid"] == pgid
                and any(lifecycle.same_process(i, r) for r in retained)]

    def settle(seconds):
        end = time.monotonic() + seconds
        while snapshot() and time.monotonic() < end:
            time.sleep(0.05)

    members = snapshot()
    if members and all(i is None for _, i in members):
        settle(RECONCILE_ABSENT_WAIT_S)  # zombies only: let launchd reap them
    for sig, wait_s in ((signal.SIGTERM, RECONCILE_GRACE_S), (signal.SIGKILL, RECONCILE_ABSENT_WAIT_S)):
        members = snapshot()
        if not members:
            break
        if not proven(members):
            return {"resolved": False, "group_absent": False, "signals": signals, "survivors": [],
                    "unproven": [i or {"pid": p, "identity": None} for p, i in members], "retained": adopted,
                    "note": "no live member matches an identity this packet recorded; nothing was signalled"}
        for _, ident in members:
            if ident and not any(lifecycle.same_process(ident, r) for r in retained):
                retained.append(ident)
                adopted.append(ident)
        try:
            os.killpg(pgid, sig)
            signals.append(signal.Signals(sig).name)
        except ProcessLookupError:
            break
        settle(wait_s)
    final = snapshot()
    return {"resolved": not final, "group_absent": not final, "signals": signals, "unproven": [],
            "survivors": [i or {"pid": p, "identity": None} for p, i in final], "retained": adopted}


def any_grant(args, plan):
    try:
        kind = json.loads(Path(args.grant).read_text()).get("kind")
    except (OSError, ValueError, AttributeError) as error:
        raise Refused(f"grant {args.grant} is absent or unreadable") from error
    if kind not in ("gpu", "build"):
        raise Refused(f"unknown grant kind {kind!r}")
    grant, _ = c.load_grant(args.grant, kind, plan)
    return grant


def command_reconcile(args, plan):
    """Packet lock only: an orphan that outlived a SIGKILLed runner may itself
    hold the inherited shared-lock descriptor. Reconcile starts no program."""
    grant = any_grant(args, plan)
    ledger = Ledger(Path(grant["evidence_dir"]))
    with ledger.locked():
        results = {}
        for label, entry in sorted(ledger.blocked().items()):
            result = reconcile_child(entry, ledger.retained_identities(entry))
            ledger.append({"type": "reconcile", "label": label, **result})
            results[label] = result
        print(json.dumps({"reconciled": results, "shared_gpu_lock": shared_lock_state(),
                          "latched_failures": ledger.failures()}, indent=1))
        unresolved = sorted(label for label, result in results.items() if not result["resolved"])
        if unresolved:
            raise Refused(f"ownership unproven or survivors remain for {unresolved}; nothing unproven was "
                          "signalled; root must handle those processes and every child stays refused")


def owned_alive(ledger):
    found = []
    for entry in ledger.children().values():
        found.extend(lifecycle.still_alive(ledger.retained_identities(entry)))
    return found


def command_release(args, plan):
    with device_session(args, plan) as session:
        evidence, ledger = session.evidence, session.ledger
        if (evidence / "released.json").exists():
            raise Refused("already released")
        if ledger.blocked():
            raise Refused(f"process groups never verified absent: {sorted(ledger.blocked())}; run reconcile")
        alive, running = owned_alive(ledger), frozen_processes(evidence)
        if alive or running:
            raise Refused(f"owned processes still alive {alive} or frozen binaries running {running}")
        totals = ledger.totals()
        children = ledger.children()
        release = {"format": "incant-0070-release-v2", "packet": c.PACKET, "released_at": now_iso(),
                   "grant_sha256": session.grant_sha, "ledger_sha256": c.sha256_file(ledger.path),
                   "children": totals["children"], "existing_frames_reserved": totals["existing_frames"],
                   "new_frames_reserved": totals["new_frames"], "pngs_reserved": totals["pngs"],
                   "retries": totals["retries"], "owned_processes_alive": alive, "frozen_processes": running,
                   "succeeded": sorted(label for label, e in children.items()
                                       if (e["complete"] or {}).get("success") is True),
                   "latched_failures": ledger.failures(),
                   "note": "GPU lease released under the shared GPU lock; the target directory is left for root."}
        write_new_json(evidence / "released.json", release)
        print(json.dumps(release, indent=1))


def command_release_build(args, plan):
    """Build lease receipt: every build child completed and nothing owned lives.
    The target directory itself is left in place for root."""
    grant, grant_sha = c.load_grant(args.grant, "build", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    with ledger.locked():
        path = evidence / "build-released.json"
        if path.exists():
            raise Refused("build lease already released")
        if ledger.blocked():
            raise Refused(f"process groups never verified absent: {sorted(ledger.blocked())}; run reconcile")
        alive = owned_alive(ledger)
        if alive:
            raise Refused(f"owned processes still alive: {alive}")
        builds = [e["complete"] for label, e in ledger.children().items()
                  if label.startswith("build-") and e["complete"]]
        receipt = {"format": "incant-0070-build-release-v2", "packet": c.PACKET, "released_at": now_iso(),
                   "grant_sha256": grant_sha, "ledger_sha256": c.sha256_file(ledger.path),
                   "build_children": [{"label": r["label"], "success": r.get("success"),
                                       "exit": (r.get("outcome") or {}).get("exit"),
                                       "group_absent": (r.get("outcome") or {}).get("group_absent")}
                                      for r in builds],
                   "build_record": (evidence / "build" / "build.json").is_file(),
                   "note": "No owned cargo/rustc process remains; the target directory is not deleted."}
        write_new_json(path, receipt)
        print(json.dumps(receipt, indent=1))


def command_status(args, plan):
    grant = json.loads(Path(args.grant).read_text())
    ledger = Ledger(Path(grant["evidence_dir"]))
    print(json.dumps({"totals": ledger.totals(), "blocked": sorted(ledger.blocked()),
                      "latched_failures": ledger.failures(),
                      "released": (Path(grant["evidence_dir"]) / "released.json").exists()}, indent=1))


COMMANDS = {"build": build, "release-build": command_release_build, "list": command_list,
            "suite": command_suite, "case": command_case, "appearance": command_appearance,
            "retry": command_retry, "release": command_release, "reconcile": command_reconcile,
            "status": command_status}


def parse(argv):
    parser = argparse.ArgumentParser(description="0070 guarded runner", add_help=True)
    parser.add_argument("command", choices=sorted(COMMANDS))
    parser.add_argument("--grant", required=True)
    parser.add_argument("--receipt")
    parser.add_argument("target", nargs="?")
    return parser.parse_args(argv)


def run_command(argv, plan):
    """Exit status: 0 success, 1 a child failed (latched), 2 refused."""
    args = parse(argv)
    try:
        completion = COMMANDS[args.command](args, plan)
    except Refused as refusal:
        record_refusal(args.grant, str(refusal), argv)
        print(f"refused: {refusal}", file=sys.stderr)
        return 2
    if isinstance(completion, dict) and completion.get("type") == "complete" and completion.get("success") is not True:
        print(f"child {completion['label']} failed {completion.get('failure_reasons')}; its evidence and "
              "reservation are retained and the packet is latched", file=sys.stderr)
        return 1
    return 0


def main(argv=None):
    lifecycle.install_signal_handlers()
    return run_command(sys.argv[1:] if argv is None else argv, c.load_plan())


if __name__ == "__main__":
    sys.exit(main())
