#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0067. Standard library only.

Adapted from handoff 0066's tools/gpu.py (read-only reuse of its
environment-safe ledger). Before every child, under an exclusive flock, it
checks: Astra's grant names this packet, its source commit and the target;
HEAD is that commit and the reviewed manifest inputs and frozen repository
paths are unchanged; the helper sources and frozen ledgers still match their
build record; this packet's release latch is absent; the frozen binary SHA-256
matches its build record (re-checked after the run). Children run one at a
time in their own process group with an explicit minimal environment (no
session variables; receipts record only allowlisted settings): 60 s for a
listing, 300 s for a regression suite, 120 s for a numeric case or the
appearance helper. Survivors in the group are killed and every child is
reaped. Numeric frame attempts (48) and appearance renders (24) are reserved
from the frozen ledgers before a child starts. Every child goes to the ledger;
every refusal to gpu-refusals.jsonl.

  gpu.py list <suite>                 # --list and --list --ignored (no GPU work)
  gpu.py suite <suite>                # all ignored cases of a regression binary, serial
  gpu.py case <name>                  # one frozen numeric case of drs_gpu
  gpu.py appearance                   # the frozen appearance plan
  gpu.py release                      # after the last child: verify none remain, write the latch

Suites: lib, model_gpu, gpu_host-lib, gpu_host-game, gpu_host-ui.
"""

import fcntl
import json
import os
import signal
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

SUITES = ("lib", "model_gpu", "gpu_host-lib", "gpu_host-game", "gpu_host-ui")


def refuse(reason, **context):
    with open(c.REFUSALS, "a") as log:
        log.write(json.dumps({"at": c.now(), "reason": reason, "argv": sys.argv[1:], **context}) + "\n")
    raise SystemExit(f"refused: {reason}")


def helper_record():
    """The latest frozen helper build (helper, then helper-r2, helper-r3 after recorded corrections)."""
    latest = c.EVIDENCE / "build" / "helper.json"
    for n in range(2, 10):
        candidate = c.EVIDENCE / "build" / f"helper-r{n}.json"
        if candidate.exists():
            latest = candidate
    return latest


def gate(helper):
    if c.LATCH.exists():
        refuse("GPU slot already released by this packet")
    try:
        grant_sha, source = c.check_grant(), c.check_source()
        lease = c.check_lease()
    except SystemExit as error:
        refuse(str(error))
    digest, _ = c.helper_digest()
    if helper and digest != c.read_json(helper_record())["helper_digest"]:
        refuse("helper sources or frozen ledgers changed since the build")
    return grant_sha, source, lease, digest


def frozen(group, name):
    record = c.EVIDENCE / "build" / "tests.json" if group == "tests" else helper_record()
    entry = c.read_json(record)["frozen"].get(name)
    if not entry:
        refuse(f"no frozen {group}/{name}")
    return Path(entry["path"]), entry["sha256"]


def ledger_entries():
    if not c.LEDGER.exists():
        return []
    return [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]


def run_child(label, kind, binary, expected, argv, timeout, attempts=0, renders=0, outputs=None, extra_env=None):
    logs = c.EVIDENCE / "gpu-logs"
    logs.mkdir(exist_ok=True)
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        grant_sha, source, lease, helper_digest = gate(kind in ("case", "appearance"))
        if c.sha(binary) != expected:
            refuse(f"{binary} does not match its frozen hash", label=label)
        entries = ledger_entries()
        used_attempts = sum(e.get("attempts_reserved", 0) for e in entries)
        used_renders = sum(e.get("renders_reserved", 0) for e in entries)
        if used_attempts + attempts > c.NUMERIC_MAX:
            refuse(f"numeric budget: {used_attempts} used, {attempts} requested", label=label)
        if used_renders + renders > c.APPEARANCE_MAX:
            refuse(f"appearance budget: {used_renders} used, {renders} requested", label=label)
        stdout_path, stderr_path = logs / f"{label}.stdout.txt", logs / f"{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            refuse(f"label {label} already used", label=label)
        env = c.run_env()
        for key, value in (extra_env or {}).items():
            if key not in c.ALLOWLIST:
                refuse(f"environment key {key} is not allowlisted", label=label)
            env[key] = value
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            child = subprocess.Popen(argv, stdout=out, stderr=err, env=env, cwd=c.EVIDENCE,
                                     start_new_session=True)
            timed_out = False
            try:
                code = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(child.pid, signal.SIGKILL)
                code = child.wait()
            finally:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        ended = c.now()
        after = c.sha(binary)
        entry = {"label": label, "kind": kind, "argv": [str(a) for a in argv],
                 "binary_sha256_before": expected, "binary_sha256_after": after, "grant_sha256": grant_sha,
                 "source": source, "helper_digest": helper_digest, "lease_acquired_at": lease["acquired_at"],
                 "env": c.recorded_env(env), "pid": child.pid, "started": started, "ended": ended,
                 "exit": code, "timed_out": timed_out, "timeout_s": timeout,
                 "attempts_reserved": attempts, "renders_reserved": renders,
                 "stdout_sha256": c.sha(stdout_path), "stderr_sha256": c.sha(stderr_path)}
        if outputs is not None:
            entry.update(outputs())
        with open(c.LEDGER, "a") as ledger:
            ledger.write(json.dumps(entry) + "\n")
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "started", "ended")}))
    if after != expected:
        raise SystemExit("binary changed during the run")
    return entry


def listing(suite):
    binary, expected = frozen("tests", suite)
    run_child(f"list-{suite}", "list", binary, expected, [str(binary), "--list"], 60)
    run_child(f"list-ignored-{suite}", "list", binary, expected, [str(binary), "--list", "--ignored"], 60)


def suite_run(suite):
    binary, expected = frozen("tests", suite)
    return run_child(f"suite-{suite}", "suite", binary, expected,
                     [str(binary), "--ignored", "--test-threads=1"], 300)


def receipts(path):
    out = []
    for line in Path(path).read_text().splitlines():
        # libtest prints the first receipt on its "test <name> ... " line.
        if "DRS-RECEIPT " in line:
            out.append(json.loads(line.split("DRS-RECEIPT ", 1)[1]))
    return out


def case(name):
    cases = {entry["name"]: entry for entry in c.read_json(c.CASES)["cases"]}
    if name not in cases:
        refuse(f"unknown case {name}")
    binary, expected = frozen("helper", "drs_gpu")
    label = f"case-{name}"
    extra_env = {}
    for key, relative in cases[name].get("env", {}).items():
        path = (c.EVIDENCE / relative).resolve()
        if c.EVIDENCE.resolve() not in path.parents:
            refuse(f"{key} must stay under the evidence directory")
        if key.endswith("_OUT") and path.exists():
            refuse(f"{path} exists; outputs are never overwritten")
        if key.endswith("_FRAMES") and not path.is_dir():
            refuse(f"{path} is missing")
        extra_env[key] = str(path)
    logs = c.EVIDENCE / "gpu-logs"

    def outputs():
        found = receipts(logs / f"{label}.stdout.txt")
        budget = [r["body"] for r in found if r["kind"] == "budget"]
        verdict = [r["body"] for r in found if r["kind"] == "verdict"]
        return {"case": name, "receipts": len(found), "budget": budget[0] if budget else None,
                "verdict": verdict[0] if verdict else None,
                "budget_matches_ledger": bool(budget) and budget[0]["attempts"] == cases[name]["attempts"]}

    return run_child(label, "case", binary, expected,
                     [str(binary), "--ignored", "--exact", name, "--test-threads=1", "--nocapture"], 120,
                     attempts=cases[name]["attempts"], outputs=outputs, extra_env=extra_env)


def appearance():
    plan = c.read_json(c.PLAN)
    renders = plan["renders"]
    for r in renders:
        if r["size"][0] > c.MAX_W or r["size"][1] > c.MAX_H:
            refuse(f"render {r['name']} exceeds {c.MAX_W}x{c.MAX_H}")
    output = c.EVIDENCE / "captures" / "appearance"
    if output.exists():
        refuse("capture output exists; captures are never overwritten")
    output.parent.mkdir(parents=True, exist_ok=True)
    binary, expected = frozen("helper", "drs_appearance")

    def outputs():
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        report = output / "report.json"
        return {"plan_sha256": c.sha(c.PLAN), "pngs": {p.name: c.sha(p) for p in pngs},
                "report_sha256": c.sha(report) if report.exists() else None}

    return run_child("appearance", "appearance", binary, expected, [str(binary), str(c.PLAN), str(output)], 120,
                     renders=len(renders), outputs=outputs)


def release():
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if c.LATCH.exists():
            raise SystemExit("already released")
        listing_text = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
        # Owned children are frozen executables themselves (argv[0] under bin/), never a shell
        # whose command line merely mentions the path.
        owned = [line.strip() for line in listing_text.splitlines()
                 if len(line.split(None, 1)) == 2 and line.split(None, 1)[1].startswith(str(c.EVIDENCE / "bin"))]
        if owned:
            raise SystemExit(f"owned processes still running: {owned}")
        entries = ledger_entries()
        alive = []
        for entry in entries:
            args = subprocess.run(["ps", "-p", str(entry["pid"]), "-o", "args="], capture_output=True,
                                  text=True).stdout.strip()
            if args.startswith(str(c.EVIDENCE / "bin")):
                alive.append(entry["pid"])
        if alive:
            raise SystemExit(f"ledger PIDs still alive: {alive}")
        refusals = [json.loads(line) for line in c.REFUSALS.read_text().splitlines()] if c.REFUSALS.exists() else []
        record = {"packet": c.PACKET, "released_at": c.now(), "children": len(entries),
                  "refusals": len(refusals),
                  "numeric_attempts_reserved": sum(e.get("attempts_reserved", 0) for e in entries),
                  "appearance_renders_reserved": sum(e.get("renders_reserved", 0) for e in entries),
                  "first_start": entries[0]["started"] if entries else None,
                  "last_end": entries[-1]["ended"] if entries else None,
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT) if c.GRANT.exists() else None,
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "Release by 0067 for Astra; review-ready.json left untouched (only Astra writes it)."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    args = sys.argv[1:]
    if len(args) == 2 and args[0] == "list" and args[1] in SUITES:
        listing(args[1])
    elif len(args) == 2 and args[0] == "suite" and args[1] in SUITES:
        suite_run(args[1])
    elif len(args) == 2 and args[0] == "case":
        case(args[1])
    elif args == ["appearance"]:
        appearance()
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
