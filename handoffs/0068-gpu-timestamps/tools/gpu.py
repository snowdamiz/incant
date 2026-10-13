#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0068. Standard library only.

Adapted from handoff 0066's committed tools/gpu.py (read-only reuse). Before
every child, under the packet flock, it checks in this order: Astra's grant
(packet, target, source commit), the packet release latch, the target lease
(same grant), committed packet tools, every child-checkout file against its git
blob, and the frozen binary SHA-256 (re-checked after the run). Budgets come
from plan.json and the ledger: <= 24 new appearance frames, <= 48 new numeric
submissions, per-item run limits. Each child runs in its own process group with
a constructed environment (HOME passed through unrecorded; PATH, TMPDIR,
RUST_BACKTRACE and the review output directory as recorded overrides), so no
regression test writes optional INCANT_* evidence. Timeouts: listings 60 s,
existing suites 300 s, the numeric test and review cases 120 s. The group is
killed and reaped; survivors are never left. Every child goes to the ledger and
every refusal (after the grant exists) to gpu-refusals.jsonl.

  gpu.py list <lib|model_gpu|game|ui|review>
  gpu.py numeric                 the brief's numeric test, exactly once
  gpu.py suite <lib|model_gpu|game|ui>   all ignored cases; lib skips the numeric test
  gpu.py review <probe|lit|sprites_text|viewport|owner_drop>
  gpu.py release                 after the last child: verify none remain, write the latch
"""

import fcntl
import json
import os
import re
import signal
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


def frozen(name):
    for record in ("tests.json", "review.json"):
        path = c.BUILD / record
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
    result = RESULT.findall(stdout)
    running = RUNNING.findall(stdout)
    return {"running": [int(n) for n in running],
            "results": [{"status": r[0], "passed": int(r[1]), "failed": int(r[2]),
                         "ignored": int(r[3]), "measured": int(r[4]), "filtered_out": int(r[5])}
                        for r in result],
            "failures": failures(stdout)}


def run_child(label, kind, item, binary_name, args, cwd, timeout, frames=0, submissions=0,
              extra=None, after=None):
    p = c.plan()
    c.LOGS.mkdir(parents=True, exist_ok=True)
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _, grant_sha = c.gate()
        tools = c.tools_identity()
        source = c.verify_checkout()
        binary, expected = frozen(binary_name)
        if c.sha(binary) != expected:
            c.refuse(f"{binary} does not match its frozen hash")
        entries = ledger()
        used_frames = sum(e.get("frames_reserved", 0) for e in entries)
        used_submissions = sum(e.get("submissions_reserved", 0) for e in entries)
        if used_frames + frames > p["budgets"]["appearance_frames_max"]:
            c.refuse(f"appearance frame budget: {used_frames} used, {frames} requested")
        if used_submissions + submissions > p["budgets"]["numeric_submissions_max"]:
            c.refuse(f"numeric submission budget: {used_submissions} used, {submissions} requested")
        stdout_path = c.LOGS / f"gpu-{label}.stdout.txt"
        stderr_path = c.LOGS / f"gpu-{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            c.refuse(f"label {label} already used")
        overrides = {"PATH": c.SYSTEM_PATH, "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)),
                     "RUST_BACKTRACE": "1", **(extra or {})}
        argv = [str(binary), *args]
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            child = subprocess.Popen(argv, cwd=c.CHECKOUT / cwd, env=c.child_env(overrides),
                                     stdout=out, stderr=err, start_new_session=True)
            timed_out = False
            original_error = None
            try:
                child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
            except BaseException:
                # Do not propagate until the child is reaped and its attempt is
                # durably charged to the ledger, including reserved GPU budgets.
                original_error = sys.exc_info()
            finally:
                # A second Ctrl-C must not interrupt kill/reap. SIGKILL of this
                # runner is inherently outside Python's cleanup guarantee.
                previous_sigint = signal.signal(signal.SIGINT, signal.SIG_IGN)
                try:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass  # also covers exit racing the timeout
                    code = child.wait()
                finally:
                    signal.signal(signal.SIGINT, previous_sigint)
        ended = c.now()
        try:
            os.killpg(child.pid, 0)
            group_empty = False
        except ProcessLookupError:
            group_empty = True
        binary_after = c.sha(binary)
        stdout = stdout_path.read_text(errors="replace")
        entry = {"label": label, "kind": kind, "item": item, "binary": binary_name,
                 "argv": argv, "cwd": str(c.CHECKOUT / cwd), "env": c.env_receipt(overrides),
                 "binary_sha256_before": expected, "binary_sha256_after": binary_after,
                 "grant_sha256": grant_sha, "tools": tools, "source": source, "pid": child.pid,
                 "started": started, "ended": ended, "exit": code, "timed_out": timed_out,
                 "wait_error": type(original_error[1]).__name__ if original_error else None,
                 "timeout_s": timeout, "group_empty_after": group_empty,
                 "frames_reserved": frames, "submissions_reserved": submissions,
                 "scratch_leftovers": sorted(os.listdir(c.SCRATCH / label)),
                 "stdout": str(stdout_path), "stdout_sha256": c.sha(stdout_path),
                 "stderr": str(stderr_path), "stderr_sha256": c.sha(stderr_path),
                 "libtest": libtest_summary(stdout)}
        if after is not None:
            try:
                entry.update(after(stdout))
            except BaseException as error:  # parsing/interrupt must not lose the attempt
                entry["after_error"] = repr(error)
                if not isinstance(error, Exception) and original_error is None:
                    original_error = sys.exc_info()
        try:
            with open(c.LEDGER, "a") as log:
                log.write(json.dumps(entry) + "\n")
                log.flush()
                os.fsync(log.fileno())
        except BaseException as ledger_error:
            if original_error:
                raise original_error[1].with_traceback(original_error[2]) from ledger_error
            raise
        if original_error:
            raise original_error[1].with_traceback(original_error[2])
        if not group_empty:
            raise SystemExit("owned GPU process group remains after cleanup; see ledger")
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "started",
                                            "ended", "libtest")}, indent=1))
    if binary_after != expected:
        raise SystemExit("binary changed during the run")
    return entry


def runs(kind, item):
    return [e for e in ledger() if e["kind"] == kind and e.get("item") == item]


def listing(name):
    p = c.plan()
    if name == "review":
        binary, cwd = p["review"]["binary"], p["review"]["cwd"]
    else:
        binary, cwd = p["suites"][name]["binary"], p["suites"][name]["cwd"]
    if runs("list", name):
        c.refuse(f"{name} already listed")

    def names(stdout):
        found = LISTING.findall(stdout)
        expected = (sorted(case["test"] for case in p["review"]["cases"].values()) if name == "review"
                    else p["suites"][name]["expected_ignored_at_reviewed_base"])
        return {"listed": found, "listed_count": len(found),
                "matches_reviewed_base": (sorted(found) == expected if name == "review"
                                          else len(found) == expected)}

    return run_child(f"list-{name}", "list", name, binary, ["--list", "--ignored"], cwd,
                     p["budgets"]["listing_timeout_s"], after=names)


def numeric():
    p = c.plan()
    spec = p["numeric"]
    lists = runs("list", "lib")
    if not lists or spec["test"] not in lists[-1]["listed"]:
        c.refuse("list lib first; the numeric test must be in the frozen library binary")
    if len(runs("numeric", "numeric")) >= spec["max_runs"]:
        c.refuse("the numeric test already ran; it never runs twice")

    def samples(stdout_unused):
        stderr = (c.LOGS / "gpu-numeric.stderr.txt").read_text(errors="replace")
        lines = [line.split("GPU timestamp result: ", 1)[1] for line in stderr.splitlines()
                 if "GPU timestamp result: " in line]
        adapter = [line for line in stderr.splitlines() if line.startswith("Timestamp adapter: ")]
        return {"samples": [json.loads(line) for line in lines], "adapter_lines": adapter}

    return run_child("numeric", "numeric", "numeric", spec["binary"], [spec["test"], *spec["args"]],
                     spec["cwd"], p["budgets"]["added_process_timeout_s"],
                     submissions=spec["submissions"], after=samples)


def suite(name):
    p = c.plan()
    spec = p["suites"][name]
    lists = runs("list", name)
    if not lists:
        c.refuse(f"list {name} first")
    if len(runs("suite", name)) >= spec["max_runs"]:
        c.refuse(f"suite {name} already ran")
    args = ["--ignored", "--test-threads=1", "--nocapture"]
    for skipped in spec["skip"]:
        args += ["--skip", skipped]
    listed = lists[-1]["listed"]
    expected_running = len([n for n in listed if not any(s in n for s in spec["skip"])])

    def check(stdout):
        summary = libtest_summary(stdout)
        return {"expected_running": expected_running,
                "skipped_names_absent_from_output": all(s not in stdout for s in spec["skip"]),
                "running_matches_listing": summary["running"] == [expected_running]}

    return run_child(f"suite-{name}", "suite", name, spec["binary"], args, spec["cwd"],
                     p["budgets"]["existing_suite_timeout_s"], after=check)


def review(case):
    p = c.plan()
    spec = p["review"]
    if case not in spec["cases"]:
        c.refuse(f"unknown review case {case}")
    details = spec["cases"][case]
    data, _ = c.grant()
    if details.get("requires_astra_review") and case not in data.get("approved_extra_cases", []):
        c.refuse(f"{case} needs Astra's review; the grant does not list it in approved_extra_cases")
    if not runs("list", "review"):
        c.refuse("list review first")
    previous = runs("review", case)
    if len(previous) >= spec["max_runs_per_case"]:
        c.refuse(f"{case} reached its run limit")
    amended = [a for a in p.get("amendments", []) if a.get("case") == case and a.get("reason")]
    if previous and not (previous[-1]["timed_out"] or previous[-1].get("report_sha256") is None
                         or len(amended) >= len(previous)):
        c.refuse(f"{case} already has a complete run; a rerun needs an infrastructure failure "
                 "or a committed plan amendment with its reason")
    output = c.CAPTURES / f"{case}-r{len(previous) + 1}"
    if output.exists():
        c.refuse(f"{output} exists; captures are never overwritten")
    c.CAPTURES.mkdir(parents=True, exist_ok=True)

    def outputs(stdout_unused):
        report = output / "report.json"
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        body = c.read_json(report) if report.exists() else {}
        return {"case": case, "output": str(output),
                "pngs": {path.name: c.sha(path) for path in pngs},
                "pngs_match_plan": sorted(path.name for path in pngs) == sorted(details["pngs"]),
                "report_sha256": c.sha(report) if report.exists() else None,
                "events_sha256": c.sha(output / "events.jsonl") if (output / "events.jsonl").exists() else None,
                "outcome": body.get("outcome"), "frames_rendered": body.get("frames_rendered"),
                "timed_submissions": body.get("timed_submissions")}

    return run_child(f"review-{case}-r{len(previous) + 1}", "review", case, spec["binary"],
                     [details["test"], *spec["args"]], spec["cwd"],
                     p["budgets"]["added_process_timeout_s"], frames=details["frames"],
                     submissions=details["submissions"],
                     extra={spec["output_env"]: str(output)}, after=outputs)


def owned_processes():
    """Running processes whose executable is one of ours: a frozen binary, the
    child checkout's cargo wrapper, or anything run from the target."""
    listing = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
    owned = []
    for line in listing.splitlines():
        parts = line.split(None, 1)
        if len(parts) == 2 and parts[1].startswith((str(c.BIN), str(c.CHECKOUT), str(c.TARGET))):
            owned.append(line.strip())
    return owned


def release():
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        c.grant()
        if c.LATCH.exists():
            raise SystemExit("already released")
        owned = owned_processes()
        if owned:
            c.refuse("owned processes still running: " + json.dumps(owned)[:400])
        entries = ledger()
        alive = []
        for entry in entries:
            found = subprocess.run(["ps", "-p", str(entry["pid"]), "-o", "args="],
                                   capture_output=True, text=True).stdout.strip()
            if found.startswith((str(c.BIN), str(c.CHECKOUT), str(c.TARGET))):
                alive.append(entry["pid"])
        if alive:
            c.refuse(f"ledger PIDs still alive: {alive}")
        refusals = c.REFUSALS.read_text().splitlines() if c.REFUSALS.exists() else []
        record = {"packet": c.PACKET, "released_at": c.now(), "children": len(entries),
                  "gpu_children": len([e for e in entries if e["kind"] != "build"]),
                  "refusals": len(refusals),
                  "frames_reserved": sum(e.get("frames_reserved", 0) for e in entries),
                  "frames_rendered": sum(e.get("frames_rendered") or 0 for e in entries),
                  "pngs_written": sum(len(e.get("pngs", {})) for e in entries),
                  "submissions_reserved": sum(e.get("submissions_reserved", 0) for e in entries),
                  "first_start": entries[0]["started"] if entries else None,
                  "last_end": entries[-1]["ended"] if entries else None,
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT),
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "GPU released by 0068 for Astra; review-ready.json left untouched "
                          "(only Astra writes it). Run lease.py release next."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    c.gate()
    args = sys.argv[1:]
    p = c.plan()
    if len(args) == 2 and args[0] == "list" and (args[1] in p["suites"] or args[1] == "review"):
        listing(args[1])
    elif args == ["numeric"]:
        numeric()
    elif len(args) == 2 and args[0] == "suite" and args[1] in p["suites"]:
        suite(args[1])
    elif len(args) == 2 and args[0] == "review":
        review(args[1])
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
