#!/usr/bin/env python3
"""Guarded serial runner for handoff 0070 (combined SSAO + bloom). Stdlib only.

Nothing runs without a root-written grant. Build and GPU grants are separate
files in the root-assigned evidence directory; either may exist alone. Every
command re-verifies the grant, the exact source (engine inputs equal to the
reviewed union, porcelain -z parsed without stripping), the helper bytes and,
for GPU work, the frozen build record and binary hashes. Each child runs under
the exclusive packet lock: reserve (fsynced) -> spawn in its own process group
-> wait with a watchdog -> finally kill + reap the group -> completion record
(even after postprocessing errors) -> unlock. Refusals never start a process.

  python3 -I -B runner.py build      --grant <evidence>/build-grant.json
  python3 -I -B runner.py release-build --grant <evidence>/build-grant.json
  python3 -I -B runner.py list       --grant <evidence>/gpu-grant.json <suite>
  python3 -I -B runner.py suite      --grant <evidence>/gpu-grant.json <suite>
  python3 -I -B runner.py case       --grant <evidence>/gpu-grant.json <numeric-case>
  python3 -I -B runner.py appearance --grant <evidence>/gpu-grant.json
  python3 -I -B runner.py retry      --grant <evidence>/gpu-grant.json --receipt <file> <case|appearance>
  python3 -I -B runner.py release    --grant <evidence>/gpu-grant.json
  python3 -I -B runner.py reconcile  --grant <evidence>/gpu-grant.json
  python3 -I -B runner.py status     --grant <evidence>/<grant>.json

Suites: lib, model_gpu, gpu_host-game, gpu_host-ui (listing must match first).
"""

import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402
import lifecycle  # noqa: E402
from ledger import Ledger, Refused, now_iso  # noqa: E402

BUILD_TIMEOUT_S = 3600
SUITE_BINARIES = ("lib", "model_gpu", "gpu_host-game", "gpu_host-ui")
HELPER_PACKAGE = "incant_0070_effects"
RECEIPT = "EFFECTS-RECEIPT "


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
    except (OSError, ValueError, TypeError):
        return
    if not evidence.is_dir() or c.EVIDENCE_ROOT not in evidence.parents or grant_path.parent != evidence:
        return
    line = json.dumps({"at": now_iso(), "reason": reason, "argv": argv}, sort_keys=True) + "\n"
    with open(evidence / "refusals.jsonl", "a") as out:
        out.write(line)
        out.flush()


# ------------------------------------------------------------------ execution

def execute(ledger, plan, request, argv, env, cwd, timeout_s, binary=None, postprocess=None):
    """Reserve, run and record one child. The caller holds the packet lock."""
    label = request["label"]
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
                                               "binary": binary})
    for path in private:
        path.mkdir(parents=True)
    outcome, error = {}, None
    completion = {"type": "complete", "label": label}
    try:
        lifecycle.run(argv, env=env, cwd=cwd, stdout_path=stdout, stderr_path=stderr,
                      timeout_s=timeout_s, outcome=outcome,
                      on_spawn=lambda pid, pgid: ledger.append(
                          {"type": "spawned", "label": label, "pid": pid, "pgid": pgid}))
    except BaseException as exc:  # interrupt or spawn failure; the group is already reaped
        error = exc
    finally:
        completion["outcome"] = outcome
        completion["error"] = None if error is None else f"{type(error).__name__}: {error}"
        for name, path in (("stdout", stdout), ("stderr", stderr)):
            completion[f"{name}_sha256"] = c.sha256_file(path) if path.exists() else None
            completion[f"{name}_bytes"] = path.stat().st_size if path.exists() else None
        if binary is not None:
            after = c.sha256_file(binary["path"])
            completion["binary_unchanged"] = after == binary["sha256"]
        if error is None and postprocess is not None:
            try:
                completion["postprocess"] = postprocess(outcome, stdout, stderr)
            except BaseException as exc:
                completion["postprocess_error"] = f"{type(exc).__name__}: {exc}"
                error = exc
        ledger.append(completion)
    if error is not None:
        raise error
    if binary is not None and not completion["binary_unchanged"]:
        raise Refused(f"{binary['path']} changed during the run")
    print(json.dumps({"label": label, "pid": outcome.get("pid"), "exit": outcome.get("exit"),
                      "signal": outcome.get("signal"), "timed_out": outcome.get("timed_out"),
                      "group_absent": outcome.get("group_absent")}))
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
    if completion["outcome"].get("exit") != 0:
        raise Refused(f"cargo {label} failed; see logs/build-{label}.stderr")
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
    grant, grant_sha = c.load_grant(args.grant, "build", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    with ledger.locked():
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
            if done["outcome"].get("exit") != 0:
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


# ------------------------------------------------------------------ GPU gate

def gpu_gate(args, plan):
    grant, grant_sha = c.load_grant(args.grant, "gpu", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    return grant, grant_sha, evidence, ledger


def gpu_checks(grant, grant_sha, evidence, ledger):
    """Re-verified under the lock before every GPU child."""
    if (evidence / "released.json").exists():
        raise Refused("the GPU lease was released by this packet; no further GPU work")
    unresolved = ledger.unresolved()
    if unresolved:
        raise Refused(f"unresolved reservations {[r['label'] for r in unresolved]}; run reconcile")
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
    return record, {"grant_sha256": grant_sha, "head": freeze["head"], "tree": freeze["tree"],
                    "dirty_or_untracked": freeze["dirty_or_untracked"], "helper_digest": digest}


def child_env(ledger, grant, label):
    return c.run_env(private_dir(ledger, "tmp", label), grant["expected_backend"])


def listing_matched(ledger, suite):
    for r in ledger.records():
        if r.get("type") == "complete" and r["label"] == f"list-{suite}":
            return bool(r.get("postprocess", {}).get("match"))
    return False


def parse_libtest(stdout_path):
    text = stdout_path.read_text(errors="replace")
    results = dict(re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", text, re.M))
    running = re.findall(r"^running (\d+) tests?$", text, re.M)
    summary = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text, re.M)
    return results, running, summary


def command_list(args, plan):
    grant, grant_sha, evidence, ledger = gpu_gate(args, plan)
    suite = args.target
    if suite not in SUITE_BINARIES:
        raise Refused(f"unknown suite {suite}")
    with ledger.locked():
        record, identity = gpu_checks(grant, grant_sha, evidence, ledger)
        binary = record["frozen"][suite]
        expected = plan["suites"][suite]["ignored"]
        label = f"list-{suite}"

        def postprocess(outcome, stdout, _stderr):
            names = re.findall(r"^(\S+): test$", stdout.read_text(errors="replace"), re.M)
            delta = {"missing": sorted(set(expected) - set(names)), "unexpected": sorted(set(names) - set(expected))}
            result = {"listed": len(names), "expected": len(expected), **delta,
                      "match": outcome.get("exit") == 0 and not delta["missing"] and not delta["unexpected"]}
            write_new_json(evidence / "listings" / f"{suite}.json", {"names": names, **result})
            return result

        return execute(ledger, plan, {"label": label, "kind": "listing", "identity": identity},
                [binary["path"], "--list", "--ignored"], child_env(ledger, grant, label),
                private_dir(ledger, "cwd", label), plan["timeouts_s"]["listing"],
                binary={"path": binary["path"], "sha256": binary["sha256"]}, postprocess=postprocess)


def command_suite(args, plan):
    grant, grant_sha, evidence, ledger = gpu_gate(args, plan)
    suite = args.target
    if suite not in SUITE_BINARIES:
        raise Refused(f"unknown suite {suite}")
    with ledger.locked():
        record, identity = gpu_checks(grant, grant_sha, evidence, ledger)
        if not listing_matched(ledger, suite):
            raise Refused(f"{suite}: no matching frozen listing; run `list {suite}` first")
        entry = plan["suites"][suite]
        allowed = [n for n in grant.get("allow_deferred_tests", []) if n in entry["deferred"]]
        run = sorted(entry["run"] + allowed)
        existing = entry["existing_frames"] + sum(entry["deferred"][n]["frames"] for n in allowed)
        label = f"suite-{suite}"

        def postprocess(outcome, stdout, _stderr):
            results, running, summary = parse_libtest(stdout)
            return {"expected": len(run), "executed": len(results), "running": running, "summary": summary,
                    "failed": sorted(n for n, r in results.items() if r == "FAILED"),
                    "missing": sorted(set(run) - set(results)), "unexpected": sorted(set(results) - set(run)),
                    "passed": sum(1 for r in results.values() if r == "ok"), "exit": outcome.get("exit")}

        binary = record["frozen"][suite]
        return execute(ledger, plan, {"label": label, "kind": "suite", "existing_frames": existing,
                               "new_frames": entry["new_frames"], "tests": run, "identity": identity},
                [binary["path"], "--ignored", "--exact", "--test-threads=1", *run],
                child_env(ledger, grant, label), private_dir(ledger, "cwd", label),
                plan["timeouts_s"]["suite"][suite],
                binary={"path": binary["path"], "sha256": binary["sha256"]}, postprocess=postprocess)


def helper_postprocess(evidence, declared, out_dir):
    def postprocess(outcome, stdout, _stderr):
        receipts = []
        for line in stdout.read_text(errors="replace").splitlines():
            if RECEIPT in line:
                receipts.append(json.loads(line.split(RECEIPT, 1)[1]))
        budget = [r["body"] for r in receipts if r.get("kind") == "budget"]
        verdict = [r["body"] for r in receipts if r.get("kind") == "verdict"]
        files = {}
        if out_dir.exists():
            for path in sorted(p for p in out_dir.rglob("*") if p.is_file()):
                files[str(path.relative_to(evidence))] = c.sha256_file(path)
        return {"receipts": len(receipts), "budget": budget, "verdict": verdict, "files": files,
                "frames_within_declaration": bool(budget) and all(b["frames"] <= b["declared"] == declared
                                                                  for b in budget),
                "exit": outcome.get("exit")}
    return postprocess


def run_helper(args, plan, kind, name, label, declared, pngs, timeout, argv_tail, precheck=None):
    grant, grant_sha, evidence, ledger = gpu_gate(args, plan)
    with ledger.locked():
        record, identity = gpu_checks(grant, grant_sha, evidence, ledger)
        if precheck is not None:
            label = precheck(ledger, evidence)
        binary = record["frozen"]["effects_gpu"]
        out_dir = evidence / "outputs" / label
        if out_dir.exists():
            raise Refused(f"{out_dir} exists; outputs are never replaced")
        out_dir.parent.mkdir(parents=True, exist_ok=True)
        return execute(ledger, plan, {"label": label, "kind": kind, "name": name, "new_frames": declared, "pngs": pngs,
                               "identity": identity},
                [binary["path"], *argv_tail, str(out_dir)], child_env(ledger, grant, label),
                private_dir(ledger, "cwd", label), timeout,
                binary={"path": binary["path"], "sha256": binary["sha256"]},
                postprocess=helper_postprocess(evidence, declared, out_dir))


def numeric_case(plan, name):
    cases = {case["name"]: case for case in plan["numeric_cases"]}
    if name not in cases:
        raise Refused(f"unknown numeric case {name}")
    return cases[name]


def command_case(args, plan):
    case = numeric_case(plan, args.target)
    return run_helper(args, plan, "numeric", case["name"], f"case-{case['name']}", case["frames"], 0,
               plan["timeouts_s"]["numeric"], ["case", case["name"]])


def command_appearance(args, plan):
    appearance = plan["appearance"]
    return run_helper(args, plan, "appearance", "appearance", "appearance", appearance["renders"], appearance["pngs"],
               plan["timeouts_s"]["appearance"], ["appearance"])


def command_retry(args, plan):
    """A focused repair retry needs a root receipt naming the original failure;
    the receipt and retry count are checked under the packet lock."""
    target = args.target
    original_label = "appearance" if target == "appearance" else f"case-{target}"
    if target == "appearance":
        declared, pngs, tail = plan["appearance"]["renders"], plan["appearance"]["pngs"], ["appearance"]
    else:
        declared, pngs, tail = numeric_case(plan, target)["frames"], 0, ["case", target]

    def precheck(ledger, evidence):
        receipt_path = Path(args.receipt or "")
        if not receipt_path.is_file() or receipt_path.resolve().parent != evidence.resolve():
            raise Refused("retry receipt must be a root-written file in the evidence directory")
        receipt = json.loads(receipt_path.read_text())
        completed = {r["label"] for r in ledger.records() if r.get("type") == "complete"}
        if receipt.get("format") != "incant-0070-retry-v1" or receipt.get("target") != target \
                or receipt.get("original_label") != original_label or original_label not in completed \
                or receipt.get("approved_by") != "root" or not receipt.get("reason"):
            raise Refused("retry receipt does not name a completed original child, a reason and root approval")
        attempt = sum(1 for r in ledger.records() if r.get("type") == "reserve" and r.get("kind") == "retry") + 1
        return f"retry{attempt}-{target}"

    return run_helper(args, plan, "retry", target, None, declared, pngs, plan["timeouts_s"]["retry"], tail,
               precheck=precheck)


def frozen_processes(evidence):
    """Live processes whose executable is a frozen binary of this packet."""
    listing = subprocess.run(["/bin/ps", "-axo", "pid=,pgid=,comm="], capture_output=True, text=True,
                             env={"PATH": c.SYSTEM_PATH, "LC_ALL": "C"}).stdout
    prefix = str((evidence / "bin").resolve()) + "/"
    found = []
    for line in listing.splitlines():
        parts = line.split(None, 2)
        if len(parts) == 3 and parts[2].startswith(prefix):
            found.append({"pid": int(parts[0]), "pgid": int(parts[1]), "command": parts[2]})
    return found


def command_reconcile(args, plan):
    grant, _, evidence, ledger = gpu_gate(args, plan)
    with ledger.locked():
        for reserve in ledger.unresolved():
            label = reserve["label"]
            spawned = [r for r in ledger.records() if r.get("type") == "spawned" and r["label"] == label]
            killed = []
            for process in frozen_processes(evidence):
                os.kill(process["pid"], signal.SIGKILL)
                killed.append(process)
            for record in spawned:
                if lifecycle.group_alive(record["pgid"]):
                    members = lifecycle.group_members(record["pgid"])
                    if members and all(lifecycle._started_after(start, reserve["at_epoch"])
                                       for _, start, _ in members):
                        os.killpg(record["pgid"], signal.SIGKILL)
                        killed.extend({"pid": m[0]} for m in members)
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and (frozen_processes(evidence) or any(
                    lifecycle.group_alive(r["pgid"]) for r in spawned)):
                time.sleep(0.1)
            survivors = frozen_processes(evidence)
            ledger.append({"type": "reconciled", "label": label, "killed": killed, "survivors": survivors,
                           "groups_absent": not any(lifecycle.group_alive(r["pgid"]) for r in spawned),
                           "note": "runner lost uncatchably; reservation stays charged"})
            print(json.dumps({"label": label, "killed": len(killed), "survivors": survivors}))


def command_release(args, plan):
    grant, grant_sha, evidence, ledger = gpu_gate(args, plan)
    with ledger.locked():
        if (evidence / "released.json").exists():
            raise Refused("already released")
        if ledger.unresolved():
            raise Refused("unresolved reservations; run reconcile first")
        records = ledger.records()
        alive = [r["pgid"] for r in records if r.get("type") == "spawned" and lifecycle.group_alive(r["pgid"])]
        running = frozen_processes(evidence)
        if alive or running:
            raise Refused(f"owned groups still alive {alive} or frozen binaries running {running}")
        totals = ledger.totals()
        release = {"format": "incant-0070-release-v1", "packet": c.PACKET, "released_at": now_iso(),
                   "grant_sha256": grant_sha, "ledger_sha256": c.sha256_file(ledger.path),
                   "children": totals["children"], "existing_frames_reserved": totals["existing_frames"],
                   "new_frames_reserved": totals["new_frames"], "pngs_reserved": totals["pngs"],
                   "retries": totals["retries"], "owned_groups_alive": alive, "frozen_processes": running,
                   "failures": [r["label"] for r in records if r.get("type") == "complete" and (
                       r.get("error") or r.get("postprocess_error")
                       or (r.get("outcome") or {}).get("exit") not in (0,))],
                   "note": "GPU lease released; the target directory is left in place for root."}
        write_new_json(evidence / "released.json", release)
        print(json.dumps(release, indent=1))


def command_release_build(args, plan):
    """Build lease receipt: every build child completed and its group is gone.
    The target directory itself is left in place for root."""
    grant, grant_sha = c.load_grant(args.grant, "build", plan)
    evidence = Path(grant["evidence_dir"])
    ledger = Ledger(evidence)
    with ledger.locked():
        path = evidence / "build-released.json"
        if path.exists():
            raise Refused("build lease already released")
        if ledger.unresolved():
            raise Refused("unresolved reservations; run reconcile first")
        records = ledger.records()
        builds = [r for r in records if r.get("type") == "complete" and r.get("label", "").startswith("build-")]
        alive = [r["pgid"] for r in records if r.get("type") == "spawned" and lifecycle.group_alive(r["pgid"])]
        if alive:
            raise Refused(f"owned build groups still alive: {alive}")
        receipt = {"format": "incant-0070-build-release-v1", "packet": c.PACKET, "released_at": now_iso(),
                   "grant_sha256": grant_sha, "ledger_sha256": c.sha256_file(ledger.path),
                   "build_children": [{"label": r["label"], "exit": (r.get("outcome") or {}).get("exit"),
                                       "group_absent": (r.get("outcome") or {}).get("group_absent")}
                                      for r in builds],
                   "build_record": (evidence / "build" / "build.json").is_file(),
                   "note": "No owned cargo/rustc group remains; the target directory is not deleted."}
        write_new_json(path, receipt)
        print(json.dumps(receipt, indent=1))


def command_status(args, plan):
    path = Path(args.grant)
    grant = json.loads(path.read_text())
    ledger = Ledger(Path(grant["evidence_dir"]))
    print(json.dumps({"totals": ledger.totals(), "unresolved": [r["label"] for r in ledger.unresolved()],
                      "released": (Path(grant["evidence_dir"]) / "released.json").exists()}, indent=1))


COMMANDS = {"build": build, "release-build": command_release_build, "list": command_list, "suite": command_suite, "case": command_case,
            "appearance": command_appearance, "retry": command_retry, "release": command_release,
            "reconcile": command_reconcile, "status": command_status}


def main(argv=None):
    lifecycle.install_signal_handlers()
    parser = argparse.ArgumentParser(description="0070 guarded runner", add_help=True)
    parser.add_argument("command", choices=sorted(COMMANDS))
    parser.add_argument("--grant", required=True)
    parser.add_argument("--receipt")
    parser.add_argument("target", nargs="?")
    args = parser.parse_args(argv)
    plan = c.load_plan()
    try:
        completion = COMMANDS[args.command](args, plan)
    except Refused as refusal:
        record_refusal(args.grant, str(refusal), sys.argv[1:] if argv is None else argv)
        print(f"refused: {refusal}", file=sys.stderr)
        return 2
    if isinstance(completion, dict) and completion.get("type") == "complete":
        outcome = completion.get("outcome") or {}
        post = completion.get("postprocess") or {}
        failed = (outcome.get("exit") != 0 or outcome.get("timed_out") or post.get("failed")
                  or post.get("missing") or post.get("unexpected") or post.get("match") is False
                  or any(v.get("failures") for v in post.get("verdict", []))
                  or ("frames_within_declaration" in post and not post["frames_within_declaration"]))
        if failed:
            print(f"child {completion['label']} failed; its evidence and reservation are retained",
                  file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
