#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0068 revision 2. Standard library only.

Adapted from handoff 0066's committed tools/gpu.py (read-only reuse) with
Astra's audited 0068 wait/kill/reap/ledger lifecycle. Before every child it
checks the grant first (nothing is created before it exists), then takes the
global native-gpu.lock (bounded wait) and the packet lock, and checks in order:
the grant (packet, target, the plan's bound source), the packet release latch,
the target lease (same grant), committed packet tools, every child-checkout file
against its git blob, and the frozen binary SHA-256 (re-checked after the run).

Budgets are debited before launch: a fsynced "debit" row reserves the attempt's
frames and numeric submissions while both locks are held, so the totals are
serialized and monotonic and an interrupted or killed attempt stays charged.
Caps come from plan.json: <= 24 new appearance frames and <= 48 new numeric
submissions, failures included, plus per-item attempt limits. Each child runs in
its own process group with a constructed environment (HOME passed through
unrecorded; PATH, TMPDIR, RUST_BACKTRACE and the review output directory as
recorded overrides), so no regression test writes optional INCANT_* evidence.
SIGINT, SIGTERM and SIGHUP during a child, and any ledger or parser exception,
end in the same unconditional kill and wait. Timeouts: listings 60 s, existing
suites 300 s, the numeric test and review cases 120 s.

  gpu.py list <lib|model_gpu|timestamp_backend_probe|game|ui|review>
  gpu.py numeric                 the brief's numeric test by full name, exactly one attempt
  gpu.py suite <lib|model_gpu|timestamp_backend_probe|game|ui>   lib skips the numeric test
  gpu.py review <probe|lit|sprites_text|viewport|cascades|owner_drop>
  gpu.py release                 after the last child: verify none remain, write the latch
"""

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


def debits():
    return [row for row in ledger() if row["kind"] == "debit"]


def attempts(kind, item):
    """Debited attempts, including any whose result row never arrived."""
    return [row for row in debits() if row["attempt_kind"] == kind and row["item"] == item]


def results(kind, item):
    return [row for row in ledger() if row["kind"] == kind and row.get("item") == item]


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
    c.grant()  # before any lock or log file can be created
    c.LOGS.mkdir(parents=True, exist_ok=True)
    with c.exclusive():
        _, grant_sha = c.gate()
        tools = c.tools_identity()
        source = c.verify_checkout()
        binary, expected = frozen(binary_name)
        if c.sha(binary) != expected:
            c.refuse(f"{binary} does not match its frozen hash")
        charged = debits()
        used_frames = sum(row["frames"] for row in charged)
        used_submissions = sum(row["submissions"] for row in charged)
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
        # Serialized, monotonic debit: durable before the child can exist.
        c.durable_append(c.LEDGER, {"kind": "debit", "label": label, "attempt_kind": kind,
                                    "item": item, "frames": frames, "submissions": submissions,
                                    "at": c.now(), "grant_sha256": grant_sha,
                                    "binary_sha256": expected})
        started = c.now()
        child = None
        timed_out = False
        original_error = None
        code = None
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err, c.terminations_raise():
            try:
                child = subprocess.Popen(argv, cwd=c.CHECKOUT / cwd, env=c.child_env(overrides),
                                         stdout=out, stderr=err, start_new_session=True)
                child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
            except BaseException:
                # Do not propagate until the child is reaped and its attempt is
                # durably recorded; the debit row already charged its budgets.
                original_error = sys.exc_info()
            finally:
                # Neither a second Ctrl-C nor a termination signal may interrupt
                # kill/reap. SIGKILL of this runner is outside Python's cleanup.
                with c.signals_ignored():
                    if child is not None:
                        try:
                            os.killpg(child.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass  # also covers exit racing the timeout
                        code = child.wait()
        ended = c.now()
        group_empty = True
        if child is not None:
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
                 "grant_sha256": grant_sha, "tools": tools, "source": source,
                 "pid": child.pid if child is not None else None,
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
            c.durable_append(c.LEDGER, entry)
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


def target(name):
    p = c.plan()
    if name == "review":
        return p["review"]["binary"], p["review"]["cwd"]
    return p["suites"][name]["binary"], p["suites"][name]["cwd"]


def listing(name):
    p = c.plan()
    binary, cwd = target(name)
    if results("list", name) or len(attempts("list", name)) >= 2:
        c.refuse(f"{name} already listed")

    def names(stdout):
        found = LISTING.findall(stdout)
        if name == "review":
            expected = sorted(case["test"] for case in p["review"]["cases"].values())
            return {"listed": found, "listed_count": len(found),
                    "matches_plan": sorted(found) == expected}
        expected = p["suites"][name]["expected_ignored"]
        return {"listed": found, "listed_count": len(found), "matches_plan": len(found) == expected}

    return run_child(f"list-{name}", "list", name, binary, ["--list", "--ignored"], cwd,
                     p["budgets"]["listing_timeout_s"], after=names)


def numeric():
    p = c.plan()
    spec = p["numeric"]
    lists = results("list", "lib")
    if not lists or spec["test"] not in lists[-1]["listed"]:
        c.refuse("list lib first; the numeric test's full name must be in the frozen library binary")
    if attempts("numeric", "numeric"):
        c.refuse("the numeric test was already attempted; it never runs twice")

    def samples(stdout):
        stderr = (c.LOGS / "gpu-numeric.stderr.txt").read_text(errors="replace")
        lines = [line.split("GPU timestamp result: ", 1)[1] for line in stderr.splitlines()
                 if "GPU timestamp result: " in line]
        adapter = [line for line in stderr.splitlines() if line.startswith("Timestamp adapter: ")]
        summary = libtest_summary(stdout)
        named = re.search(r"^test " + re.escape(spec["test"]) + r" \.\.\. ", stdout, re.M) is not None
        one = (summary["running"] == [1] and len(summary["results"]) == 1
               and summary["results"][0]["passed"] + summary["results"][0]["failed"] == 1
               and summary["results"][0]["ignored"] == 0)
        return {"exactly_one_test_ran": one and named, "named_test_line": named,
                "test_passed": one and named and summary["results"][0]["passed"] == 1,
                "samples": [json.loads(line) for line in lines], "adapter_lines": adapter}

    return run_child("numeric", "numeric", "numeric", spec["binary"], [spec["test"], *spec["args"]],
                     spec["cwd"], p["budgets"]["added_process_timeout_s"],
                     submissions=spec["submissions"], after=samples)


def suite(name):
    p = c.plan()
    spec = p["suites"][name]
    lists = results("list", name)
    if not lists:
        c.refuse(f"list {name} first")
    if attempts("suite", name):
        c.refuse(f"suite {name} was already attempted; no uncontrolled retry")
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
    if not results("list", "review"):
        c.refuse("list review first")
    tried = attempts("review", case)
    done = results("review", case)
    if len(tried) >= spec["max_runs_per_case"]:
        c.refuse(f"{case} reached its attempt limit")
    amended = [a for a in p.get("amendments", []) if a.get("case") == case and a.get("reason")]
    infrastructure = len(done) < len(tried) or (done and (done[-1]["timed_out"]
                                                          or done[-1].get("report_sha256") is None))
    if tried and not (infrastructure or len(amended) >= len(tried)):
        c.refuse(f"{case} already has a complete run; a rerun needs an infrastructure failure "
                 "or a committed plan amendment with its reason")
    output = c.CAPTURES / f"{case}-r{len(tried) + 1}"
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

    return run_child(f"review-{case}-r{len(tried) + 1}", "review", case, spec["binary"],
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
    c.grant()
    with c.exclusive():
        if c.LATCH.exists():
            raise SystemExit("already released")
        owned = owned_processes()
        if owned:
            c.refuse("owned processes still running: " + json.dumps(owned)[:400])
        rows = ledger()
        children = [row for row in rows if row["kind"] != "debit"]
        alive = []
        for entry in children:
            if entry.get("pid") is None:
                continue
            found = subprocess.run(["ps", "-p", str(entry["pid"]), "-o", "args="],
                                   capture_output=True, text=True).stdout.strip()
            if found.startswith((str(c.BIN), str(c.CHECKOUT), str(c.TARGET))):
                alive.append(entry["pid"])
        if alive:
            c.refuse(f"ledger PIDs still alive: {alive}")
        charged = [row for row in rows if row["kind"] == "debit"]
        unresolved = sorted({d["label"] for d in charged} - {e["label"] for e in children})
        refusals = c.REFUSALS.read_text().splitlines() if c.REFUSALS.exists() else []
        record = {"packet": c.PACKET + "-r2", "released_at": c.now(), "children": len(children),
                  "gpu_children": len([e for e in children if e["kind"] != "build"]),
                  "debited_attempts": len(charged), "attempts_without_result": unresolved,
                  "refusals": len(refusals),
                  "frames_debited": sum(d["frames"] for d in charged),
                  "frames_rendered": sum(e.get("frames_rendered") or 0 for e in children),
                  "pngs_written": sum(len(e.get("pngs", {})) for e in children),
                  "submissions_debited": sum(d["submissions"] for d in charged),
                  "first_start": children[0]["started"] if children else None,
                  "last_end": children[-1]["ended"] if children else None,
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT),
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "GPU released by 0068-r2 for Astra; review-ready.json left untouched "
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
