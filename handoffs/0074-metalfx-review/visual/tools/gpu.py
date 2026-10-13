#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0074 stage B. Standard library only.

Adapted from handoff 0072's committed visual/tools/gpu.py (visual tree ac8a48de,
read-only reuse) without its regression suites: only the frozen MetalFX review
binary runs. For every child, under the packet flock and then the global
native-gpu.lock (never waited for), it checks in order: root's grant, the stop
and release latches, the lease of the same grant, the committed visual tree,
every child-checkout file against its blob, the frozen binary's hash and
execute bit. Review cases are debited (fsynced, never refunded) before launch
under the same lock, one run per case in plan order, never retried. Each child
runs in its own process group with a constructed environment and is SIGKILLed
and reaped after exit, timeout, Ctrl-C, SIGTERM or SIGHUP. Every child goes to
the ledger (rows always carry "pid"); any failure latches stop.json.

  gpu.py list review  list the frozen binary's ignored cases (no GPU work)
  gpu.py review <case>  the plan's cases, in plan order (late_stationary, late_motion)
  gpu.py release      after the last child, also when stopped: write the GPU latch
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

LISTING = re.compile(r"^(.+): test$", re.M)
RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                    r"(\d+) measured; (\d+) filtered out", re.M)
RUNNING = re.compile(r"^running (\d+) tests?$", re.M)


def ledger():
    if not c.LEDGER.exists():
        return []
    return [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]


def runs(kind, item):
    return [e for e in ledger() if e.get("kind") == kind and e.get("item") == item]


def frozen(name):
    path = c.BUILD / "review.json"
    if path.exists():
        entry = c.read_json(path)["frozen"].get(name)
        if entry:
            return Path(entry["path"]), entry["sha256"]
    c.refuse(f"no frozen binary {name}")


def failures(stdout):
    """Names in libtest's closing failures: list (FAILED can sit on its own line
    under --nocapture, so per-line status parsing is not trusted)."""
    if "\nfailures:\n" not in stdout:
        return []
    names = []
    for line in stdout.rsplit("\nfailures:\n", 1)[1].splitlines():
        if not line.strip():
            if names:
                break
            continue
        if not line.startswith("    "):
            break
        names.append(line.strip())
    return names


def libtest_summary(stdout):
    return {"running": [int(n) for n in RUNNING.findall(stdout)],
            "results": [{"status": r[0], "passed": int(r[1]), "failed": int(r[2]),
                         "ignored": int(r[3]), "measured": int(r[4]), "filtered_out": int(r[5])}
                        for r in RESULT.findall(stdout)],
            "failures": failures(stdout)}


def run_child(label, kind, item, binary_name, args, cwd, timeout, charge=None, extra=None,
              after=None):
    """charge = (frames, ui_submissions, pngs) debited before launch, or None."""
    p = c.plan()
    data, _ = c.gate()  # refuses before any lock or file when the grant is absent
    c.LOGS.mkdir(parents=True, exist_ok=True)
    with c.packet_lock(), c.gpu_lock(), c.deferred_signals():
        data, grant_sha = c.gate()
        tools = c.tools_identity(data)
        source = c.verify_checkout()
        binary, expected = frozen(binary_name)
        if c.sha(binary) != expected:
            c.refuse(f"{binary} does not match its frozen hash")
        c.require_executable(binary)
        stdout_path = c.LOGS / f"gpu-{label}.stdout.txt"
        stderr_path = c.LOGS / f"gpu-{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            c.refuse(f"label {label} already used")
        overrides = {"PATH": c.SYSTEM_PATH, "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)),
                     "RUST_BACKTRACE": "1", "INCANT_EXPECT_GPU_BACKEND": p["expected_backend"],
                     **(extra or {})}
        debit = c.debit(label, item, *charge, grant_sha) if charge else None
        argv = [str(binary), *args]
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            pid, code, timed_out, original, group_empty = c.supervise(
                argv, c.CHECKOUT / cwd, c.child_env(overrides), out, err, timeout)
        stdout = stdout_path.read_text(errors="replace")
        entry = {"label": label, "kind": kind, "item": item, "binary": binary_name, "argv": argv,
                 "cwd": str(c.CHECKOUT / cwd), "env": c.env_receipt(overrides),
                 "binary_sha256_before": expected, "binary_sha256_after": c.sha(binary),
                 "grant_sha256": grant_sha, "tools": tools, "source": source, "debit": debit,
                 "pid": pid, "started": started, "ended": c.now(), "exit": code,
                 "timed_out": timed_out,
                 "wait_error": type(original[1]).__name__ if original else None,
                 "timeout_s": timeout, "group_empty_after": group_empty,
                 "scratch_leftovers": sorted(x.name for x in (c.SCRATCH / label).iterdir()),
                 "stdout": str(stdout_path), "stdout_sha256": c.sha(stdout_path),
                 "stderr": str(stderr_path), "stderr_sha256": c.sha(stderr_path),
                 "libtest": libtest_summary(stdout)}
        failed = []
        if after is not None:
            try:
                extra_fields = after(stdout)
                entry.update(extra_fields)
                failed += [k for k, v in extra_fields.get("contract", {}).items() if not v]
            except Exception as error:  # parsing must not lose the attempt
                entry["after_error"] = repr(error)
                failed.append("after_error")
        if code != 0:
            failed.append(f"exit {code}")
        for flag in ("timed_out", "wait_error"):
            if entry[flag]:
                failed.append(flag)
        if not group_empty:
            failed.append("group not empty")
        if entry["binary_sha256_after"] != expected:
            failed.append("binary changed")
        entry["failed"] = failed
        c.append_durable(c.LEDGER, entry)
        if failed:
            c.latch_stop(label, ", ".join(failed))
        if original:
            raise original[1].with_traceback(original[2])
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "failed", "libtest")},
                     indent=1))
    if failed:
        raise SystemExit(f"{label} failed and latched stop.json: {failed}")
    return entry


def listing():
    """`--list --ignored` of the frozen binary: exactly the planned cases, no GPU work.
    The plan's list_args may add a name filter, so earlier stages' frozen cases in
    the same binary stay unlisted and unrunnable."""
    p = c.plan()
    spec = p["review"]
    if runs("list", "review"):
        c.refuse("review already listed")

    def names(stdout):
        found = LISTING.findall(stdout)
        return {"listed": found, "listed_count": len(found),
                "contract": {"matches_plan": sorted(found) == sorted(spec["cases"])}}

    return run_child("list-review", "list", "review", spec["binary"],
                     spec.get("list_args", ["--list", "--ignored"]),
                     spec["cwd"], p["budgets"]["listing_timeout_s"], after=names)


def review(case):
    p = c.plan()
    spec = p["review"]
    cases = list(spec["cases"])
    if case not in spec["cases"]:
        c.refuse(f"unknown review case {case}")
    if not runs("list", "review"):
        c.refuse("list review first")
    if len(runs("review", case)) >= spec["max_runs_per_case"]:
        c.refuse(f"{case} already ran; reruns need a root amendment and grant")
    for earlier in cases[:cases.index(case)]:
        previous = runs("review", earlier)
        if not previous or previous[-1]["failed"]:
            c.refuse(f"run {earlier} first; cases run in plan order and stop on failure")
    details = spec["cases"][case]
    output = c.CAPTURES / case
    if output.exists():
        c.refuse(f"{output} exists; captures are never overwritten")
    c.CAPTURES.mkdir(parents=True, exist_ok=True)

    def outputs(stdout_unused):
        report = output / "report.json"
        body = c.read_json(report) if report.exists() else {}
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        return {"case": case, "output": str(output),
                "pngs": {path.name: c.sha(path) for path in pngs},
                "report_sha256": c.sha(report) if report.exists() else None,
                "events_sha256": c.sha(output / "events.jsonl") if (output / "events.jsonl").exists() else None,
                "outcome": body.get("outcome"), "frames_rendered": body.get("frames_rendered"),
                "ui_submissions": body.get("ui_submissions"), "checks": body.get("checks"),
                "contract": {"report_pass": body.get("outcome") == "pass",
                             "pngs_match_plan": sorted(x.name for x in pngs) == sorted(details["pngs"]),
                             "frames_within_debit": (body.get("frames_rendered") or 0) <= details["frames"]
                             and (body.get("ui_submissions") or 0) <= details["ui_submissions"]}}

    return run_child(f"review-{case}", "review", case, spec["binary"], [case, *spec["args"]],
                     spec["cwd"], p["budgets"]["review_case_timeout_s"],
                     charge=(details["frames"], details["ui_submissions"], len(details["pngs"])),
                     extra={spec["output_env"]: str(output)}, after=outputs)


def owned_processes(target):
    listing_text = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
    owned = []
    for line in listing_text.splitlines():
        parts = line.split(None, 1)
        if len(parts) == 2 and parts[1].startswith((str(c.BIN), str(c.CHECKOUT), str(target))):
            owned.append(line.strip())
    return owned


def release():
    data, _ = c.grant()
    with c.packet_lock():
        if c.LATCH.exists():
            raise SystemExit("already released")
        owned = owned_processes(c.target(data))
        if owned:
            c.refuse("owned processes still running: " + json.dumps(owned)[:400])
        entries = ledger()
        alive = []
        for entry in entries:
            found = subprocess.run(["ps", "-p", str(entry["pid"]), "-o", "args="],
                                   capture_output=True, text=True).stdout.strip()
            if found.startswith((str(c.BIN), str(c.CHECKOUT), str(c.target(data)))):
                alive.append(entry["pid"])
        if alive:
            c.refuse(f"ledger PIDs still alive: {alive}")
        used = c.budget_used()
        record = {"packet": c.PACKET, "stage": c.STAGE, "released_at": c.now(),
                  "children": len(entries),
                  "gpu_children": len([e for e in entries if e["kind"] != "build"]),
                  "stopped": c.read_json(c.STOP) if c.STOP.exists() else None,
                  "debited": used, "debits": len(c.budget_rows()),
                  "frames_rendered": sum(e.get("frames_rendered") or 0 for e in entries),
                  "ui_submissions": sum(e.get("ui_submissions") or 0 for e in entries),
                  "pngs_written": sum(len(e.get("pngs", {})) for e in entries),
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "budget_sha256": c.sha(c.BUDGET) if c.BUDGET.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT),
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "GPU released for root; review-ready.json untouched. Run lease.py release next."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    args = sys.argv[1:]
    if args == ["release"]:
        release()
        return
    c.gate()
    if args == ["list", "review"]:
        listing()
    elif len(args) == 2 and args[0] == "review":
        review(args[1])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
