#!/usr/bin/env python3
"""Four ordered frozen owner-history numerical/retained tests. Preparation grants no GPU use.

check PLAN is read-only. run PLAN requires the fresh root-issued grant in the
fixed evidence directory. No listing/build/retry is performed by this runner.
The prior reviewed runner supplies only bounded process and I/O primitives.
"""
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve()
LIFECYCLE = HERE.parent.parent / "owner/owner_runner.py"
LIFECYCLE_SHA = "b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34"
if hashlib.sha256(LIFECYCLE.read_bytes()).hexdigest() != LIFECYCLE_SHA:
    raise SystemExit("refused: reviewed lifecycle source changed")
spec = importlib.util.spec_from_file_location("owner_lifecycle", LIFECYCLE)
life = importlib.util.module_from_spec(spec)
spec.loader.exec_module(life)
TEST = ("upscaling::system::owner_device_tests::"
        "actual_owner_history_obeys_provenance_and_preserves_ordinary_fallback")
MARKER = ("OWNER_HISTORY cases=12 owner_renders=12 current_only_renders=1 baseline_renders=2 "
          "queue_submits=12 readback_maps=12 max_extent=16x16 no_images=true")
RETAINED_TEST = ("models::retained::upscaling_owner_tests::"
                 "retained_owner_color_depth_records_commit_and_reset_together")
RETAINED_MARKER = ("RETAINED_OWNER frames=12 readback_submits=12 queue_submits=24 "
                   "maps=12 max_extent=32x32 no_images=true")
KINDS = ["wgsl", "cooked-metal"]
# Closed ordered jobs: the retained children cannot run before both kernel tests pass.
JOBS = [("numeric-wgsl", 0, TEST, 12), ("numeric-cooked-metal", 1, TEST, 12),
        ("retained-wgsl", 0, RETAINED_TEST, 24),
        ("retained-cooked-metal", 1, RETAINED_TEST, 24)]
E = Path.home() / ".codex/incant-evidence/0073-owner-history-gpu-002"
GLOBAL_LOCK = E.parent / "native-gpu.lock"
TIMEOUT = 90
ENV = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "TMPDIR": "<fresh evidence root>/tmp",
       "INCANT_EXPECT_GPU_BACKEND": "Metal", "RUST_BACKTRACE": "0"}
BUDGET = {"child_attempts": 4, "numeric_cases": 24, "numeric_owner_renders": 24,
          "numeric_current_only_renders": 2, "numeric_baseline_renders": 4,
          "numeric_depth_clear_passes": 24, "retained_renderer_frames": 24,
          "retained_readback_submissions": 24, "queue_submissions": 72,
          "readback_maps": 48, "maximum_extent": [32, 32],
          "timeout_seconds_each": TIMEOUT, "images": 0, "builds": 0, "retries": 0}
SOURCE_HASHES = {
    "shader": "0618bd7793f2ef53db7d34bc2020ba515bb722b5a3e5729912cf1f5c5c408777",
    "common": "7a1041707465239b046b31ae9d86e87f4372635056622eeacb8a3142e105344a",
    "device_test": "e6c2028f47ce61c30a0c67c1de69ec0c077ddc260b6f86ba6cd256c5ef2d2ac7",
}
SOURCE_HASHES["retained_test"] = "8cd2a178688990aeea07d576a1e11b0b5ff82a559fecebce5bab1d8fc037358d"
ROLES = set(SOURCE_HASHES) | {"source_freeze", "cpu_review", "fixture_review", "retained_review", "wgsl_build",
                            "cooked-metal_build", "wgsl_list", "cooked-metal_list"}
PLAN_KEYS = {"schema", "state", "producer", "evidence_dir", "budget", "environment",
             "identities", "binaries"}
GRANT_KEYS = {"schema", "authority", "holder", "issued_at", "plan_sha256",
              "runner_sha256", "lifecycle_sha256", "binary_sha256", "evidence_dir",
              "predecessor_release_path", "predecessor_release_sha256", "budget"}
CASE = re.compile(r"^OWNER_HISTORY case=(\d+) center=.* probes=(\d+) baseline=(true|false) current_only=(true|false)$")


def require(ok, message):
    if not ok:
        raise life.Refused("refused: " + message)


def identity(row):
    require(set(row) == {"path", "sha256"}, "identity fields")
    p = Path(row["path"])
    require(p.is_absolute() and p.is_file() and not p.is_symlink(), "regular absolute identity")
    require(life.HEX64.fullmatch(row["sha256"]) and life.sha(p) == row["sha256"], "identity hash")
    return p


def read_plan(path):
    p = Path(path)
    require(p.is_absolute() and p.is_file() and not p.is_symlink(), "plan path")
    data = json.loads(p.read_text())
    require(set(data) == PLAN_KEYS, "plan fields")
    require(data["schema"] == "incant-owner-history-combined-plan-v2"
            and data["state"] == "FROZEN_READY_FOR_ROOT_GRANT", "preparation is not frozen")
    require(bool(life.HEX40.fullmatch(data["producer"])), "producer identity")
    require(data["evidence_dir"] == str(E), "evidence directory")
    require(data["budget"] == BUDGET and data["environment"] == ENV, "fixed budget/environment")
    require(set(data["identities"]) == ROLES, "identity roles")
    for role, row in data["identities"].items():
        ip = identity(row)
        if role in SOURCE_HASHES:
            require(row["sha256"] == SOURCE_HASHES[role], role + " source changed")
        if role.endswith("_list"):
            lines = ip.read_text().splitlines()
            require(all(lines.count(t + ": test") == 1 for t in (TEST, RETAINED_TEST)),
                    "both exact test names enumerated once")
    require(len(data["binaries"]) == 2, "two binaries required")
    for kind, row in zip(KINDS, data["binaries"]):
        require(set(row) == {"kind", "path", "sha256", "bytes", "mode"}, "binary fields")
        require(row["kind"] == kind and row["mode"] == "0555", "binary kind/mode")
        bp = Path(row["path"])
        require(bp.is_absolute() and E.parent in bp.parents
                and not any("target" in part for part in bp.parts), "outside-target frozen binary")
        life.check_binary({k: v for k, v in row.items() if k != "kind"})
    require(len({x["path"] for x in data["binaries"]}) == 2
            and len({x["sha256"] for x in data["binaries"]}) == 2, "distinct backend binaries")
    return data


def grant(plan_path, plan):
    gp = E / "grant.json"
    require(gp.is_file() and not gp.is_symlink(), "no root grant; preparation only")
    g = json.loads(gp.read_text())
    require(set(g) == GRANT_KEYS, "grant fields")
    require(g["schema"] == "incant-owner-history-combined-grant-v2"
            and g["authority"] == "root" and g["holder"] == "Claude0073 Opus5.5 Max",
            "root authority and exact holder")
    require(isinstance(g["issued_at"], str) and g["issued_at"].startswith("20"), "issue date")
    require(g["plan_sha256"] == life.sha(plan_path) and g["runner_sha256"] == life.sha(HERE)
            and g["lifecycle_sha256"] == LIFECYCLE_SHA, "grant source binding")
    require(g["binary_sha256"] == [x["sha256"] for x in plan["binaries"]]
            and g["evidence_dir"] == str(E) and g["budget"] == BUDGET, "grant execution bounds")
    prior = life.check_predecessor(g["predecessor_release_path"], g["predecessor_release_sha256"])
    return g, prior


def judge(stdout, stderr, record, test):
    results = life.RESULT.findall(stdout)
    prefix = "test " + test + " ... "
    normalized = [x.removeprefix(prefix) for x in stdout.splitlines()]
    numeric = test == TEST
    tag = "OWNER_HISTORY" if numeric else "RETAINED_OWNER"
    lines = [x for x in normalized if x.startswith(tag + " ")]
    adapter_line = lines[0] if lines and lines[0].startswith(tag + " adapter=AdapterInfo {") else None
    adapter = adapter_line.split("adapter=", 1)[1] if adapter_line else None
    marker = MARKER if numeric else RETAINED_MARKER
    checks = {
        "successful_process": record.get("exit") == 0 and record.get("reaped")
            and record.get("group_empty_after") and not record.get("timed_out")
            and not record.get("interrupted_by") and "exception" not in record
            and "launch_error" not in record,
        "one_test": "running 1 test" in stdout.splitlines() and len(results) == 1
            and results[0][:5] == ("ok", "1", "0", "0", "0"),
        "metal_adapter": adapter is not None and "backend: Metal" in adapter,
        "final_marker": bool(lines) and lines[-1] == marker and lines.count(marker) == 1,
        "no_test_failure": "\nfailures:\n" not in stdout and "panicked at" not in stderr,
    }
    if numeric:
        cases = [CASE.fullmatch(x) for x in lines[1:-1]]
        checks.update(
            exact_case_sequence=len(lines) == 14 and len(cases) == 12 and all(cases)
                and [int(m[1]) for m in cases if m] == list(range(1, 13)),
            baseline_cases=len(cases) == 12 and all(cases)
                and [int(m[1]) for m in cases if m and m[3] == "true"] == [7, 8],
            current_only_case=len(cases) == 12 and all(cases)
                and [int(m[1]) for m in cases if m and m[4] == "true"] == [3])
    else:
        checks["exact_retained_lines"] = len(lines) == 2
    return all(checks.values()), checks, adapter


def child(row, job, env, directory):
    directory.mkdir()
    out, err = directory / "stdout.txt", directory / "stderr.txt"
    argv = [row["path"], "--ignored", "--exact", job[2], "--nocapture", "--test-threads=1"]
    with open(out, "x") as stdout, open(err, "x") as stderr:
        record, caught = life.launch(argv, directory, env, stdout, stderr, TIMEOUT)
    # launch has already killed/reaped on BaseException. Analysis failures still
    # retain that process record so the outer finally can prove lease release.
    result = {**record, "kind": row["kind"], "job": job[0], "argv": argv, "cwd": str(directory),
              "environment": life.env_receipt(env), "accepted": False}
    try:
        ok, checks, adapter = judge(out.read_text(errors="replace"), err.read_text(errors="replace"), record, job[2])
        survivors = life.owned_processes(row["path"], record["pgid"])
        checks.update(binary_unchanged=life.sha(row["path"]) == row["sha256"],
                      no_survivors=not survivors, no_images=not life.images_under(directory))
        result.update(accepted=ok and all(checks.values()), checks=checks, adapter=adapter,
                      survivors=survivors, stdout_sha256=life.sha(out), stderr_sha256=life.sha(err))
    except BaseException as error:
        result["analysis_error"] = repr(error)
        caught = caught or error
    return result, caught


def execute(plan_path):
    # No grant means no evidence directory/lock creation and no process launch.
    require((E / "grant.json").is_file(), "no root grant; preparation only")
    plan = read_plan(plan_path)
    g, prior = grant(plan_path, plan)
    with open(E / "owner.lock", "a") as lock, open(GLOBAL_LOCK, "a") as global_lock:
        for f in (lock, global_lock):
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise life.Refused("refused: exclusive GPU lock already held") from None
        require(set(os.listdir(E)) <= {"grant.json", "owner.lock"}, "evidence is not fresh; no retry")
        for row in plan["binaries"]:
            require(not life.owned_processes(row["path"]), "frozen binary already running")
        rows, caught, accepted = [], None, False
        with life.blocked():
            try:
                (E / "tmp").mkdir()
                env = life.child_env(ENV, E / "tmp")
                for job in JOBS:
                    row = plan["binaries"][job[1]]
                    life.check_binary({k: v for k, v in row.items() if k != "kind"})
                    life.append_synced(E / "debits.jsonl", {"at": life.now(), "kind": row["kind"], "job": job[0],
                        "child_attempts": 1, "queue_submissions": job[3], "readback_maps": 12,
                        "grant_sha256": life.sha(E / "grant.json"), "refundable": False})
                    result, failure = child(row, job, env, E / job[0])
                    rows.append(result)
                    if len(rows) > 1 and result.get("adapter") != rows[0].get("adapter"):
                        result["accepted"] = False
                        result["adapter_mismatch"] = True
                    life.append_synced(E / "ledger.jsonl", result)
                    if failure:
                        raise failure
                    if not result["accepted"]:
                        break
                accepted = len(rows) == 4 and all(x["accepted"] for x in rows)
            except BaseException as error:
                caught = error
            finally:
                survivors = []
                for row in plan["binaries"]:
                    survivors.extend(life.owned_processes(row["path"]))
                groups = [x["pgid"] for x in rows if x.get("pgid") and life.group_alive(x["pgid"])]
                images = life.images_under(E)
                clean = not survivors and not groups and all(x["reaped"] for x in rows)
                release = {"released": clean, "at": life.now(), "accepted": accepted and not images,
                    "plan_sha256": life.sha(plan_path), "runner_sha256": life.sha(HERE),
                    "lifecycle_sha256": LIFECYCLE_SHA, "grant_sha256": life.sha(E / "grant.json"),
                    "predecessor": prior, "children": rows, "owned_processes": survivors,
                    "process_groups_alive": groups, "images": images,
                    "exception": repr(caught) if caught else None,
                    "debits_sha256": life.sha(E / "debits.jsonl") if (E / "debits.jsonl").exists() else None,
                    "ledger_sha256": life.sha(E / "ledger.jsonl") if (E / "ledger.jsonl").exists() else None,
                    "os": {"uname": list(os.uname())}}
                if clean:
                    release["released_at"] = release["at"]
                life.write_new_json(E / ("release.json" if clean else "release-blocked.json"), release)
        if caught:
            raise caught
        require(clean, "owned process survived; release blocked")
        return release


def main():
    for sig in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, life.raise_interrupt)
    require(len(sys.argv) == 3 and sys.argv[1] in ("check", "run"), "usage: runner.py check|run /absolute/plan.json")
    path = Path(sys.argv[2])
    if sys.argv[1] == "check":
        plan = read_plan(path)
        state = grant(path, plan)[0] if (E / "grant.json").exists() else "absent; no GPU authority"
        print(json.dumps({"plan_sha256": life.sha(path), "runner_sha256": life.sha(HERE), "grant": state}))
        return 0
    result = execute(path)
    print(json.dumps(result, indent=2))
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    sys.exit(main())
